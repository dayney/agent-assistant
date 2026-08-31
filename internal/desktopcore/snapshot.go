// Package desktopcore exposes the read-only JSON boundary used by the desktop
// client. It reuses the canonical agentsync loader so the desktop view cannot
// drift from CLI semantics. It deliberately returns references, never resolved
// secret values.
package desktopcore

import (
	"errors"
	"fmt"
	"net/url"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"github.com/spf13/afero"
	"github.com/spxrogers/agentsync/internal/adapter"
	"github.com/spxrogers/agentsync/internal/adapter/codex"
	"github.com/spxrogers/agentsync/internal/adapter/cursor"
	"github.com/spxrogers/agentsync/internal/adapter/gemini"
	"github.com/spxrogers/agentsync/internal/governance"
	"github.com/spxrogers/agentsync/internal/paths"
	"github.com/spxrogers/agentsync/internal/project"
	"github.com/spxrogers/agentsync/internal/source"
	"github.com/spxrogers/agentsync/internal/state"
)

type ScopeKind string

const (
	ScopeGlobal  ScopeKind = "global"
	ScopeAgent   ScopeKind = "agent"
	ScopeProject ScopeKind = "project"
)

type CapabilityState string

const (
	CapabilityFull        CapabilityState = "full"
	CapabilityPartial     CapabilityState = "partial"
	CapabilityUnsupported CapabilityState = "unsupported"
)

type CapabilityReport struct {
	Memory     CapabilityState            `json:"memory"`
	Components map[string]CapabilityState `json:"components"`
	Note       string                     `json:"note,omitempty"`
}

type AgentSummary struct {
	ID           string           `json:"id"`
	Name         string           `json:"name"`
	Vendor       string           `json:"vendor"`
	Version      string           `json:"version"`
	Health       string           `json:"health"`
	SyncState    string           `json:"syncState"`
	Capabilities CapabilityReport `json:"capabilities"`
	LastApplied  string           `json:"lastApplied"`
}

type GlobalRuleItem struct {
	ID          string    `json:"id"`
	Name        string    `json:"name"`
	Description string    `json:"description"`
	Enabled     bool      `json:"enabled"`
	Source      ScopeKind `json:"source"`
}

type GlobalMCPItem struct {
	ID          string    `json:"id"`
	Name        string    `json:"name"`
	Description string    `json:"description"`
	Transport   string    `json:"transport"`
	Endpoint    string    `json:"endpoint"`
	SecretRefs  []string  `json:"secretRefs"`
	Source      ScopeKind `json:"source"`
}

type ProjectSummary struct {
	ID        string   `json:"id"`
	Name      string   `json:"name"`
	Path      string   `json:"path"`
	Profile   string   `json:"profile"`
	Stack     []string `json:"stack"`
	Agents    []string `json:"agents"`
	SyncState string   `json:"syncState"`
	UpdatedAt string   `json:"updatedAt"`
}

type ActivityItem struct {
	ID        string `json:"id"`
	Title     string `json:"title"`
	Detail    string `json:"detail"`
	Timestamp string `json:"timestamp"`
	Tone      string `json:"tone"`
}

type WorkspaceSnapshot struct {
	SchemaVersion int              `json:"schemaVersion"`
	Mode          string           `json:"mode"`
	GeneratedAt   string           `json:"generatedAt"`
	Metrics       Metrics          `json:"metrics"`
	Global        Global           `json:"global"`
	Agents        []AgentSummary   `json:"agents"`
	Projects      []ProjectSummary `json:"projects"`
	Activity      []ActivityItem   `json:"activity"`
}

type Metrics struct {
	Agents     int `json:"agents"`
	Projects   int `json:"projects"`
	Components int `json:"components"`
	Attention  int `json:"attention"`
}

type Global struct {
	CanonicalState string           `json:"canonicalState"`
	CanonicalPath  string           `json:"canonicalPath"`
	Rules          []GlobalRuleItem `json:"rules"`
	MCP            []GlobalMCPItem  `json:"mcp"`
	Skills         int              `json:"skills"`
	Hooks          int              `json:"hooks"`
	Subagents      int              `json:"subagents"`
}

type ApplyPreview struct {
	Files       int      `json:"files"`
	Partial     int      `json:"partial"`
	Unsupported int      `json:"unsupported"`
	Warnings    []string `json:"warnings"`
}

type Options struct {
	Home         string
	ProjectsRoot string
	NativeRoot   string
}

// ReadSnapshot loads the user's canonical source and project trees. ProjectsRoot
// is scanned only one directory deep and can be set to a specific workspace
// root by the desktop app, keeping discovery bounded and predictable.
func ReadSnapshot(opts Options) (WorkspaceSnapshot, error) {
	if opts.Home == "" {
		opts.Home = paths.AgentsyncHome(paths.OSEnv{})
	}
	if opts.ProjectsRoot == "" {
		if configured := os.Getenv("AGENT_ASSISTANT_PROJECTS_ROOT"); configured != "" {
			opts.ProjectsRoot = configured
		} else {
			home := paths.HomeDir(paths.OSEnv{})
			opts.ProjectsRoot = filepath.Join(home, "git", "work")
		}
	}
	if opts.NativeRoot == "" {
		opts.NativeRoot = paths.HomeDir(paths.OSEnv{})
	}

	global, err := source.Load(afero.NewOsFs(), opts.Home)
	if err != nil {
		return WorkspaceSnapshot{}, fmt.Errorf("load global agentsync source: %w", err)
	}
	projects, err := discoverProjects(opts.ProjectsRoot)
	if err != nil {
		return WorkspaceSnapshot{}, err
	}
	registry, err := state.LoadProjectRegistry(filepath.Join(opts.Home, ".state", "agent-assistant", "projects.json"))
	if err != nil {
		return WorkspaceSnapshot{}, fmt.Errorf("load imported projects: %w", err)
	}
	projects = mergeProjectRoots(projects, registry.Projects)

	snapshot := WorkspaceSnapshot{
		SchemaVersion: 1,
		Mode:          "real",
		GeneratedAt:   time.Now().UTC().Format(time.RFC3339),
		Global:        globalSummary(global, opts.Home),
		Projects:      make([]ProjectSummary, 0, len(projects)),
		Activity:      []ActivityItem{},
	}
	if native, nativeErr := readNativeGlobal(opts.NativeRoot); nativeErr != nil {
		return WorkspaceSnapshot{}, nativeErr
	} else {
		snapshot.Global.Rules = append(snapshot.Global.Rules, native.Rules...)
		snapshot.Global.MCP = append(snapshot.Global.MCP, native.MCP...)
	}
	agentSources := map[string]source.Agent{}
	for name, agent := range global.Config.Agents {
		if agent.Enabled {
			agentSources[name] = agent
		}
	}
	for _, root := range projects {
		canonical, loadErr := source.Load(afero.NewOsFs(), project.Home(root))
		if loadErr != nil {
			return WorkspaceSnapshot{}, fmt.Errorf("load project source %s: %w", root, loadErr)
		}
		for name, agent := range canonical.Config.Agents {
			if agent.Enabled {
				agentSources[name] = agent
			}
		}
		profile, profileOK := governance.Profile(filepath.Base(root))
		projectSummary := ProjectSummary{
			ID:        filepath.Base(root),
			Name:      filepath.Base(root),
			Path:      root,
			Profile:   filepath.Base(root),
			Stack:     []string{},
			Agents:    enabledAgentNames(canonical.Config.Agents),
			SyncState: "discovered",
			UpdatedAt: "本机读取",
		}
		if profileOK {
			projectSummary.Profile = profile.ID
			projectSummary.Stack = append(projectSummary.Stack, profile.Stack...)
		}
		if len(projectSummary.Agents) == 0 {
			projectSummary.SyncState = "not-configured"
		}
		snapshot.Projects = append(snapshot.Projects, projectSummary)
		snapshot.Activity = append(snapshot.Activity, ActivityItem{
			ID:        "project-" + projectSummary.ID,
			Title:     projectSummary.Name + " 已读取",
			Detail:    fmt.Sprintf("从 %s 读取项目 Profile 和 Agent 配置", root),
			Timestamp: "本次启动",
			Tone:      "success",
		})
	}
	sort.Slice(snapshot.Projects, func(i, j int) bool { return snapshot.Projects[i].Name < snapshot.Projects[j].Name })
	for name := range agentSources {
		report, ok := governance.Capabilities(name)
		if !ok {
			report = governance.CapabilityReport{Components: map[string]governance.CapabilityState{}}
		}
		summary := agentSummary(name, report)
		snapshot.Agents = append(snapshot.Agents, summary)
	}
	sort.Slice(snapshot.Agents, func(i, j int) bool { return snapshot.Agents[i].Name < snapshot.Agents[j].Name })
	snapshot.Metrics.Agents = len(snapshot.Agents)
	snapshot.Metrics.Projects = len(snapshot.Projects)
	snapshot.Metrics.Components = len(snapshot.Global.Rules) + len(snapshot.Global.MCP) + snapshot.Global.Skills + snapshot.Global.Hooks + snapshot.Global.Subagents
	for _, agent := range snapshot.Agents {
		if agent.Health != "good" {
			snapshot.Metrics.Attention++
		}
	}
	return snapshot, nil
}

func mergeProjectRoots(discovered []string, registered []state.RegisteredProject) []string {
	seen := make(map[string]bool, len(discovered)+len(registered))
	out := make([]string, 0, len(discovered)+len(registered))
	for _, root := range discovered {
		clean := filepath.Clean(root)
		if !seen[clean] {
			seen[clean] = true
			out = append(out, clean)
		}
	}
	for _, item := range registered {
		clean := filepath.Clean(item.Path)
		if seen[clean] {
			continue
		}
		if info, err := os.Stat(clean); err == nil && info.IsDir() {
			seen[clean] = true
			out = append(out, clean)
		}
	}
	sort.Strings(out)
	return out
}

func discoverProjects(root string) ([]string, error) {
	entries, err := os.ReadDir(root)
	if err != nil {
		if errors.Is(err, os.ErrNotExist) {
			return nil, nil
		}
		return nil, fmt.Errorf("discover projects under %s: %w", root, err)
	}
	projects := make([]string, 0)
	for _, entry := range entries {
		if !entry.IsDir() {
			continue
		}
		candidate := filepath.Join(root, entry.Name())
		info, statErr := os.Stat(project.Home(candidate))
		if statErr == nil && info.IsDir() {
			projects = append(projects, candidate)
		}
	}
	return projects, nil
}

type nativeGlobalSummary struct {
	Rules []GlobalRuleItem
	MCP   []GlobalMCPItem
}

func globalSummary(canonical source.Canonical, home string) Global {
	canonicalState := "empty"
	if _, err := os.Stat(filepath.Join(home, "agentsync.toml")); err == nil {
		canonicalState = "ready"
	} else if errors.Is(err, os.ErrNotExist) {
		canonicalState = "missing"
	}
	global := Global{CanonicalState: canonicalState, CanonicalPath: home, Rules: []GlobalRuleItem{}, MCP: []GlobalMCPItem{}}
	if strings.TrimSpace(canonical.Memory.Body) != "" {
		global.Rules = append(global.Rules, GlobalRuleItem{
			ID:          "memory",
			Name:        "全局记忆",
			Description: "来自 ~/.agentsync/memory/AGENTS.md 的公共 Agent 规则。",
			Enabled:     true,
			Source:      ScopeGlobal,
		})
	}
	for _, server := range canonical.MCPServers {
		global.MCP = append(global.MCP, GlobalMCPItem{
			ID:          server.ID,
			Name:        server.ID,
			Description: "来自 canonical MCP 配置的本地定义。",
			Transport:   server.Server.Type,
			Endpoint:    safeEndpoint(server.Server),
			SecretRefs:  secretRefs(server.Server),
			Source:      ScopeGlobal,
		})
	}
	global.Skills = len(canonical.Skills)
	global.Hooks = len(canonical.Hooks)
	global.Subagents = len(canonical.Subagents)
	return global
}

func readNativeGlobal(userHome string) (nativeGlobalSummary, error) {
	summary := nativeGlobalSummary{}
	for _, item := range []struct {
		name string
		path string
	}{
		{name: "Codex 全局规则", path: filepath.Join(userHome, ".codex", "AGENTS.md")},
		{name: "Gemini 全局规则", path: filepath.Join(userHome, ".gemini", "GEMINI.md")},
		{name: "Gemini Agent 规则", path: filepath.Join(userHome, ".gemini", "config", "AGENTS.md")},
	} {
		if _, err := os.Stat(item.path); err == nil {
			summary.Rules = append(summary.Rules, GlobalRuleItem{ID: strings.ToLower(strings.ReplaceAll(item.name, " ", "-")), Name: item.name, Description: "来自本机 Agent 原生全局文件，尚未写入 ~/.agentsync canonical。", Enabled: true, Source: ScopeAgent})
		} else if !errors.Is(err, os.ErrNotExist) {
			return nativeGlobalSummary{}, fmt.Errorf("inspect native rule %s: %w", item.path, err)
		}
	}

	for _, item := range []struct {
		name   string
		ingest func() (source.Canonical, error)
	}{
		{name: "Codex", ingest: func() (source.Canonical, error) {
			return codex.New(codex.Options{TargetRoot: userHome}).Ingest(adapter.ScopeUser, "")
		}},
		{name: "Cursor", ingest: func() (source.Canonical, error) {
			return cursor.New(cursor.Options{TargetRoot: userHome}).Ingest(adapter.ScopeUser, "")
		}},
		{name: "Gemini CLI", ingest: func() (source.Canonical, error) {
			return gemini.New(gemini.Options{TargetRoot: userHome}).Ingest(adapter.ScopeUser, "")
		}},
	} {
		canonical, err := item.ingest()
		if err != nil {
			return nativeGlobalSummary{}, fmt.Errorf("read %s native configuration: %w", item.name, err)
		}
		for _, server := range canonical.MCPServers {
			transport := server.Server.Type
			if transport == "" {
				if server.Server.URL != "" {
					transport = "http"
				} else {
					transport = "stdio"
				}
			}
			summary.MCP = append(summary.MCP, GlobalMCPItem{
				ID:          item.name + ":" + server.ID,
				Name:        item.name + " / " + server.ID,
				Description: "来自本机 Agent 原生 MCP 配置，建议迁移到 canonical 后再统一应用。",
				Transport:   transport,
				Endpoint:    safeEndpoint(server.Server),
				SecretRefs:  secretRefs(server.Server),
				Source:      ScopeAgent,
			})
		}
	}
	return summary, nil
}

func safeEndpoint(spec source.MCPServerSpec) string {
	if spec.URL != "" {
		parsed, err := url.Parse(spec.URL)
		if err == nil && parsed.Scheme != "" && parsed.Host != "" {
			return parsed.Scheme + "://" + parsed.Host
		}
		return "远程端点（已脱敏）"
	}
	if spec.Command != "" {
		return filepath.Base(spec.Command)
	}
	return spec.Type
}

func secretRefs(spec source.MCPServerSpec) []string {
	refs := make([]string, 0)
	for _, values := range []map[string]string{spec.Env, spec.Headers} {
		for _, value := range values {
			if strings.HasPrefix(value, "${secret:") || strings.HasPrefix(value, "${env:") {
				refs = append(refs, value)
			}
		}
	}
	sort.Strings(refs)
	return refs
}

func enabledAgentNames(agents map[string]source.Agent) []string {
	names := make([]string, 0, len(agents))
	for name, agent := range agents {
		if agent.Enabled {
			names = append(names, name)
		}
	}
	sort.Strings(names)
	return names
}

func agentSummary(name string, report governance.CapabilityReport) AgentSummary {
	components := make(map[string]CapabilityState, len(report.Components))
	health := "good"
	for component, state := range report.Components {
		components[component] = CapabilityState(state)
		if state != governance.Full {
			health = "warn"
		}
	}
	return AgentSummary{
		ID:           name,
		Name:         displayAgentName(name),
		Vendor:       "本机适配器",
		Version:      "已检测",
		Health:       health,
		SyncState:    healthToSync(health),
		Capabilities: CapabilityReport{Memory: CapabilityState(report.Components["rules"]), Components: components, Note: report.Summary},
		LastApplied:  "未读取应用记录",
	}
}

func healthToSync(health string) string {
	if health == "good" {
		return "discovered"
	}
	return "attention"
}

func displayAgentName(name string) string {
	switch name {
	case "codex":
		return "Codex"
	case "cursor":
		return "Cursor"
	case "gemini":
		return "Gemini CLI"
	case "antigravity":
		return "Antigravity"
	default:
		return name
	}
}

func PreviewApply(snapshot WorkspaceSnapshot) ApplyPreview {
	preview := ApplyPreview{Warnings: []string{}}
	preview.Files = snapshot.Metrics.Components + len(snapshot.Agents) + len(snapshot.Projects)
	for _, agent := range snapshot.Agents {
		for _, state := range agent.Capabilities.Components {
			switch state {
			case CapabilityPartial:
				preview.Partial++
			case CapabilityUnsupported:
				preview.Unsupported++
			}
		}
	}
	for _, agent := range snapshot.Agents {
		if agent.Health != "good" {
			preview.Warnings = append(preview.Warnings, agent.Name+" 存在能力差异")
		}
	}
	return preview
}
