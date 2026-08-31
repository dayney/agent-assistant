package desktopcore

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"github.com/sergi/go-diff/diffmatchpatch"
	"github.com/spf13/afero"

	"github.com/spxrogers/agentsync/internal/adapter"
	"github.com/spxrogers/agentsync/internal/adapterregistry"
	"github.com/spxrogers/agentsync/internal/drift"
	"github.com/spxrogers/agentsync/internal/iox"
	"github.com/spxrogers/agentsync/internal/paths"
	"github.com/spxrogers/agentsync/internal/project"
	"github.com/spxrogers/agentsync/internal/render"
	"github.com/spxrogers/agentsync/internal/secrets"
	"github.com/spxrogers/agentsync/internal/source"
	"github.com/spxrogers/agentsync/internal/state"
)

type RuleScope string

const (
	RuleScopeGlobal  RuleScope = "global"
	RuleScopeProject RuleScope = "project"
)

type RuleResolution string

const (
	RuleResolutionNone            RuleResolution = ""
	RuleResolutionBackupOverwrite RuleResolution = "backup-overwrite"
)

type RuleRequest struct {
	Scope       RuleScope `json:"scope"`
	ProjectPath string    `json:"projectPath,omitempty"`
	Agents      []string  `json:"agents,omitempty"`
}

type SaveRuleRequest struct {
	RuleRequest
	Body string `json:"body"`
}

type SyncRulesRequest struct {
	RuleRequest
	Resolution RuleResolution `json:"resolution,omitempty"`
}

type ImportNativeRuleRequest struct {
	RuleRequest
	Agent string `json:"agent"`
}

type RuleDocument struct {
	Scope         RuleScope `json:"scope"`
	ProjectPath   string    `json:"projectPath,omitempty"`
	CanonicalPath string    `json:"canonicalPath"`
	Body          string    `json:"body"`
	Fragments     []string  `json:"fragments"`
	Agents        []string  `json:"agents"`
}

type RuleTarget struct {
	Agent     string `json:"agent"`
	Path      string `json:"path,omitempty"`
	Supported bool   `json:"supported"`
	Status    string `json:"status"`
	Blocked   bool   `json:"blocked"`
	WillWrite bool   `json:"willWrite"`
	Diff      string `json:"diff,omitempty"`
	Reason    string `json:"reason,omitempty"`
}

type RuleWorkspace struct {
	Document        RuleDocument `json:"document"`
	Targets         []RuleTarget `json:"targets"`
	AvailableAgents []string     `json:"availableAgents"`
	Blocked         bool         `json:"blocked"`
}

type RuleBackup struct {
	Agent      string `json:"agent"`
	SourcePath string `json:"sourcePath"`
	BackupPath string `json:"backupPath"`
}

type RuleSyncResult struct {
	Preview RuleWorkspace `json:"preview"`
	Applied bool          `json:"applied"`
	Backups []RuleBackup  `json:"backups"`
}

type ProjectRuleSource struct {
	Path   string   `json:"path"`
	Agents []string `json:"agents"`
	Body   string   `json:"body"`
}

type ProjectRuleImport struct {
	Path          string              `json:"path"`
	Sources       []ProjectRuleSource `json:"sources"`
	NeedsAnalysis bool                `json:"needsAnalysis"`
}

type RuleProposal struct {
	Body       string   `json:"body"`
	Notes      []string `json:"notes"`
	Analyzer   string   `json:"analyzer"`
	RequiresAI bool     `json:"requiresAI"`
}

type RuleAnalyzer interface {
	Analyze(context.Context, string, []ProjectRuleSource) (RuleProposal, error)
}

type RuleCore struct {
	opts     Options
	registry *adapter.Registry
	analyzer RuleAnalyzer
}

func NewRuleCore(opts Options, analyzer RuleAnalyzer) *RuleCore {
	opts = normalizeOptions(opts)
	return &RuleCore{
		opts:     opts,
		registry: adapterregistry.New(opts.NativeRoot),
		analyzer: analyzer,
	}
}

func normalizeOptions(opts Options) Options {
	if opts.Home == "" {
		opts.Home = paths.AgentsyncHome(paths.OSEnv{})
	}
	if opts.NativeRoot == "" {
		opts.NativeRoot = paths.HomeDir(paths.OSEnv{})
	}
	if opts.ProjectsRoot == "" {
		opts.ProjectsRoot = filepath.Join(opts.NativeRoot, "git", "work")
	}
	return opts
}

func (c *RuleCore) GetRules(req RuleRequest) (RuleWorkspace, error) {
	return c.previewRules(req)
}

func (c *RuleCore) SaveRule(req SaveRuleRequest) (RuleDocument, error) {
	if strings.TrimSpace(req.Body) == "" {
		return RuleDocument{}, fmt.Errorf("rule mother template cannot be empty")
	}
	var out RuleDocument
	err := c.withLock(func() error {
		home, projectPath, err := c.canonicalHome(req.RuleRequest)
		if err != nil {
			return err
		}
		canonical, err := source.Load(afero.NewOsFs(), home)
		if err != nil {
			return fmt.Errorf("load canonical rule: %w", err)
		}
		if canonical.Config.Agents == nil {
			canonical.Config.Agents = map[string]source.Agent{}
		}
		configChanged := false
		for _, name := range req.Agents {
			if c.registry.Lookup(name) == nil {
				return fmt.Errorf("unknown Agent %q", name)
			}
			if _, exists := canonical.Config.Agents[name]; !exists {
				canonical.Config.Agents[name] = source.Agent{Enabled: true}
				configChanged = true
			}
		}
		if _, err := os.Stat(filepath.Join(home, "agentsync.toml")); errors.Is(err, os.ErrNotExist) || configChanged {
			if err := source.WriteConfig(home, canonical.Config); err != nil {
				return fmt.Errorf("write canonical config: %w", err)
			}
		} else if err != nil {
			return fmt.Errorf("inspect canonical config: %w", err)
		}
		canonical.Memory.Body = req.Body
		if err := source.WriteMemory(home, canonical.Memory); err != nil {
			return fmt.Errorf("write rule mother template: %w", err)
		}
		out = ruleDocument(req.RuleRequest, home, projectPath, canonical.Memory, selectedAgents(canonical.Config, req.Agents))
		return nil
	})
	return out, err
}

func (c *RuleCore) SyncRules(req SyncRulesRequest) (RuleSyncResult, error) {
	result := RuleSyncResult{Backups: []RuleBackup{}}
	err := c.withLock(func() error {
		preview, plan, st, sc, projectPath, err := c.previewPlan(req.RuleRequest)
		if err != nil {
			return err
		}
		result.Preview = preview
		if len(preview.Targets) == 0 {
			return fmt.Errorf("no target Agents selected for Rule synchronization")
		}
		if preview.Blocked && req.Resolution != RuleResolutionBackupOverwrite {
			return nil
		}
		if preview.Blocked {
			for _, target := range preview.Targets {
				if target.Blocked && !target.Supported {
					return fmt.Errorf("agent %s cannot receive this Rule at %s scope; deselect it before synchronizing", target.Agent, req.Scope)
				}
				if !target.Blocked || target.Status == drift.ForeignCollision.String() {
					continue
				}
				backupPath, err := render.BackupFile(c.opts.Home, target.Path)
				if err != nil {
					return err
				}
				result.Backups = append(result.Backups, RuleBackup{Agent: target.Agent, SourcePath: target.Path, BackupPath: backupPath})
			}
		}

		collisions, written, _, err := render.Apply(plan, c.registry, st, c.opts.Home, c.opts.NativeRoot, sc, projectPath)
		if err != nil {
			c.recordWrittenRuleState(st, plan, written, sc, projectPath)
			return err
		}
		for _, collision := range collisions {
			result.Backups = append(result.Backups, RuleBackup{Agent: collision.Agent, SourcePath: collision.Path, BackupPath: collision.BackupTo})
		}
		for agent, agentPlan := range plan.PerAgent {
			if err := render.RecordOpsState(st, c.opts.NativeRoot, agent, sc, projectPath, memoryOps(agentPlan.Ops)); err != nil {
				return err
			}
		}
		if err := state.Save(c.targetsStatePath(), st); err != nil {
			return err
		}
		result.Applied = true
		result.Preview, err = c.previewRules(req.RuleRequest)
		return err
	})
	return result, err
}

func (c *RuleCore) ImportNativeRule(req ImportNativeRuleRequest) (RuleDocument, error) {
	if req.Agent == "" {
		return RuleDocument{}, fmt.Errorf("agent is required")
	}
	var out RuleDocument
	err := c.withLock(func() error {
		workspace, err := c.previewRules(req.RuleRequest)
		if err != nil {
			return err
		}
		var target *RuleTarget
		for i := range workspace.Targets {
			if workspace.Targets[i].Agent == req.Agent {
				target = &workspace.Targets[i]
				break
			}
		}
		if target == nil || target.Path == "" {
			return fmt.Errorf("agent %s has no Rule destination at %s scope", req.Agent, req.Scope)
		}
		data, err := os.ReadFile(target.Path)
		if err != nil {
			return fmt.Errorf("read native Rule %s: %w", target.Path, err)
		}
		body := source.StripManagedBanner(string(data))
		memory, hadMarkers, err := source.CollapseMemoryMarkers(body)
		if err != nil {
			return fmt.Errorf("collapse native Rule fragments: %w", err)
		}
		if !hadMarkers {
			memory = source.Memory{Body: body}
		}
		home, projectPath, err := c.canonicalHome(req.RuleRequest)
		if err != nil {
			return err
		}
		if err := source.WriteMemory(home, memory); err != nil {
			return err
		}
		canonical, err := source.Load(afero.NewOsFs(), home)
		if err != nil {
			return err
		}
		out = ruleDocument(req.RuleRequest, home, projectPath, canonical.Memory, selectedAgents(canonical.Config, req.Agents))
		return nil
	})
	return out, err
}

func (c *RuleCore) ImportProject(path string) (ProjectRuleImport, error) {
	projectPath, err := validateProjectRoot(path)
	if err != nil {
		return ProjectRuleImport{}, err
	}
	var result ProjectRuleImport
	err = c.withLock(func() error {
		registryPath := c.projectRegistryPath()
		registry, err := state.LoadProjectRegistry(registryPath)
		if err != nil {
			return err
		}
		found := false
		for _, item := range registry.Projects {
			if item.Path == projectPath {
				found = true
				break
			}
		}
		if !found {
			registry.Projects = append(registry.Projects, state.RegisteredProject{Path: projectPath, AddedAt: time.Now().UTC()})
			sort.Slice(registry.Projects, func(i, j int) bool { return registry.Projects[i].Path < registry.Projects[j].Path })
			if err := state.SaveProjectRegistry(registryPath, registry); err != nil {
				return err
			}
		}
		sources, err := c.discoverProjectRuleSources(projectPath)
		if err != nil {
			return err
		}
		result = ProjectRuleImport{Path: projectPath, Sources: sources, NeedsAnalysis: uniqueRuleBodies(sources) > 1}
		return nil
	})
	return result, err
}

func (c *RuleCore) AnalyzeProjectRules(ctx context.Context, path string) (RuleProposal, error) {
	projectPath, err := validateProjectRoot(path)
	if err != nil {
		return RuleProposal{}, err
	}
	sources, err := c.discoverProjectRuleSources(projectPath)
	if err != nil {
		return RuleProposal{}, err
	}
	if len(sources) == 0 {
		return RuleProposal{}, fmt.Errorf("no native project Rule files were found")
	}
	if uniqueRuleBodies(sources) == 1 {
		return RuleProposal{
			Body:     sources[0].Body,
			Notes:    []string{"All discovered native Rule files are semantically identical by content."},
			Analyzer: "deterministic",
		}, nil
	}
	if c.analyzer == nil {
		return RuleProposal{RequiresAI: true}, fmt.Errorf("native Rule files differ and no AI analyzer is available")
	}
	proposal, err := c.analyzer.Analyze(ctx, projectPath, sources)
	if err != nil {
		return RuleProposal{}, err
	}
	if strings.TrimSpace(proposal.Body) == "" {
		return RuleProposal{}, fmt.Errorf("AI analyzer returned an empty Rule proposal")
	}
	return proposal, nil
}

func (c *RuleCore) previewRules(req RuleRequest) (RuleWorkspace, error) {
	workspace, _, _, _, _, err := c.previewPlan(req)
	return workspace, err
}

func (c *RuleCore) previewPlan(req RuleRequest) (RuleWorkspace, render.RenderPlan, *state.Targets, adapter.Scope, string, error) {
	home, projectPath, err := c.canonicalHome(req)
	if err != nil {
		return RuleWorkspace{}, render.RenderPlan{}, nil, adapter.ScopeUser, "", err
	}
	canonical, err := source.Load(afero.NewOsFs(), home)
	if err != nil {
		return RuleWorkspace{}, render.RenderPlan{}, nil, adapter.ScopeUser, "", fmt.Errorf("load canonical Rule: %w", err)
	}
	agents := selectedAgents(canonical.Config, req.Agents)
	for _, name := range agents {
		if c.registry.Lookup(name) == nil {
			return RuleWorkspace{}, render.RenderPlan{}, nil, adapter.ScopeUser, "", fmt.Errorf("unknown Agent %q", name)
		}
	}
	sc := adapter.ScopeUser
	model := source.Canonical{Config: canonical.Config, Memory: canonical.Memory}
	if req.Scope == RuleScopeProject {
		sc = adapter.ScopeProject
		projectOnly := source.Canonical{Config: canonical.Config, Memory: canonical.Memory}
		model.Project = &projectOnly
	}
	st, err := state.Load(c.targetsStatePath())
	if err != nil {
		return RuleWorkspace{}, render.RenderPlan{}, nil, sc, projectPath, err
	}
	plan, err := render.Plan(secrets.ForRender(model), c.registry, agents, sc, projectPath, st, c.opts.NativeRoot)
	if err != nil {
		return RuleWorkspace{}, render.RenderPlan{}, nil, sc, projectPath, err
	}
	for name, result := range plan.PerAgent {
		result.Ops = memoryOps(result.Ops)
		result.Skips = memorySkips(result.Skips)
		plan.PerAgent[name] = result
	}
	doc := ruleDocument(req, home, projectPath, canonical.Memory, agents)
	workspace := RuleWorkspace{
		Document:        doc,
		Targets:         make([]RuleTarget, 0, len(agents)),
		AvailableAgents: c.registry.Names(),
	}
	for _, name := range agents {
		result := plan.PerAgent[name]
		if len(result.Ops) == 0 {
			target := RuleTarget{Agent: name, Status: "empty", Supported: true}
			probeOp, probeSkip, probeErr := c.probeMemoryTarget(name, sc, projectPath)
			if probeErr != nil {
				return RuleWorkspace{}, render.RenderPlan{}, nil, sc, projectPath, probeErr
			}
			if len(result.Skips) > 0 || probeSkip != nil {
				target.Status = "unsupported"
				target.Supported = false
				target.Blocked = true
				if len(result.Skips) > 0 {
					target.Reason = result.Skips[0].Reason
				} else {
					target.Reason = probeSkip.Reason
				}
				workspace.Blocked = true
			} else if probeOp != nil {
				target.Path = probeOp.Path
				data, readErr := os.ReadFile(probeOp.Path)
				switch {
				case readErr == nil && strings.TrimSpace(string(data)) != "":
					target.Status = "native-only"
					target.Blocked = true
					target.Diff = ruleDiff(string(data), "")
					workspace.Blocked = true
				case readErr != nil && !errors.Is(readErr, os.ErrNotExist):
					return RuleWorkspace{}, render.RenderPlan{}, nil, sc, projectPath, fmt.Errorf("read native Rule %s: %w", probeOp.Path, readErr)
				}
			}
			workspace.Targets = append(workspace.Targets, target)
			continue
		}
		op := result.Ops[0]
		target, err := c.classifyRuleTarget(name, sc, projectPath, op, st)
		if err != nil {
			return RuleWorkspace{}, render.RenderPlan{}, nil, sc, projectPath, err
		}
		workspace.Blocked = workspace.Blocked || target.Blocked
		workspace.Targets = append(workspace.Targets, target)
	}
	return workspace, plan, st, sc, projectPath, nil
}

func (c *RuleCore) probeMemoryTarget(agentName string, sc adapter.Scope, projectPath string) (*adapter.FileOp, *adapter.Skip, error) {
	falseValue := false
	model := source.Canonical{
		Config: source.Config{Memory: source.MemoryConfig{Banner: &falseValue}},
		Memory: source.Memory{Body: "# agent-assistant Rule path probe\n"},
	}
	if sc == adapter.ScopeProject {
		projectOnly := model
		projectOnly.Project = nil
		model.Project = &projectOnly
	}
	a := c.registry.Lookup(agentName)
	if a == nil {
		return nil, nil, fmt.Errorf("unknown Agent %q", agentName)
	}
	ops, skips, err := a.Render(secrets.ForRender(model), sc, projectPath)
	if err != nil {
		return nil, nil, fmt.Errorf("resolve %s Rule target: %w", agentName, err)
	}
	for _, op := range memoryOps(ops) {
		copy := op
		return &copy, nil, nil
	}
	for _, skip := range memorySkips(skips) {
		copy := skip
		return nil, &copy, nil
	}
	return nil, nil, nil
}

func (c *RuleCore) classifyRuleTarget(agentName string, sc adapter.Scope, projectPath string, op adapter.FileOp, st *state.Targets) (RuleTarget, error) {
	srcHash := sha256String(op.Content)
	portableProject := paths.HomeRelative(c.opts.NativeRoot, projectPath)
	portablePath := paths.HomeRelative(c.opts.NativeRoot, op.Path)
	key := fmt.Sprintf("%s:%s:%s:%s", agentName, sc.String(), portableProject, portablePath)
	appliedHash := ""
	if entry, ok := st.Files[key]; ok {
		appliedHash = entry.SHA256
	}
	destHash := ""
	actual := ""
	data, err := os.ReadFile(op.Path)
	if err == nil {
		destHash = sha256String(data)
		actual = string(data)
	} else if !errors.Is(err, os.ErrNotExist) {
		return RuleTarget{}, fmt.Errorf("read native Rule %s: %w", op.Path, err)
	}
	class := drift.Classify(srcHash, appliedHash, destHash)
	blocked := !drift.SafeForAutoApply(class)
	return RuleTarget{
		Agent:     agentName,
		Path:      op.Path,
		Supported: true,
		Status:    class.String(),
		Blocked:   blocked,
		WillWrite: srcHash != destHash,
		Diff:      ruleDiff(actual, string(op.Content)),
	}, nil
}

func (c *RuleCore) canonicalHome(req RuleRequest) (string, string, error) {
	switch req.Scope {
	case "", RuleScopeGlobal:
		return c.opts.Home, "", nil
	case RuleScopeProject:
		projectPath, err := validateProjectRoot(req.ProjectPath)
		if err != nil {
			return "", "", err
		}
		return project.Home(projectPath), projectPath, nil
	default:
		return "", "", fmt.Errorf("unsupported Rule scope %q", req.Scope)
	}
}

func (c *RuleCore) discoverProjectRuleSources(projectPath string) ([]ProjectRuleSource, error) {
	falseValue := false
	probe := source.Canonical{
		Config: source.Config{Memory: source.MemoryConfig{Banner: &falseValue}},
		Memory: source.Memory{Body: "# agent-assistant Rule path probe\n"},
	}
	projectProbe := probe
	projectProbe.Project = nil
	probe.Project = &projectProbe
	resolved := secrets.ForRender(probe)
	byPath := map[string]*ProjectRuleSource{}
	for _, name := range c.registry.Names() {
		a := c.registry.Lookup(name)
		ops, _, err := a.Render(resolved, adapter.ScopeProject, projectPath)
		if err != nil {
			return nil, fmt.Errorf("resolve %s project Rule path: %w", name, err)
		}
		for _, op := range memoryOps(ops) {
			item := byPath[op.Path]
			if item == nil {
				item = &ProjectRuleSource{Path: op.Path}
				byPath[op.Path] = item
			}
			item.Agents = append(item.Agents, name)
		}
	}
	out := make([]ProjectRuleSource, 0, len(byPath))
	for _, item := range byPath {
		data, err := os.ReadFile(item.Path)
		if errors.Is(err, os.ErrNotExist) {
			continue
		}
		if err != nil {
			return nil, fmt.Errorf("read project Rule %s: %w", item.Path, err)
		}
		item.Body = flattenRenderedRule(source.StripManagedBanner(string(data)))
		sort.Strings(item.Agents)
		out = append(out, *item)
	}
	sort.Slice(out, func(i, j int) bool { return out[i].Path < out[j].Path })
	return out, nil
}

func (c *RuleCore) withLock(fn func() error) error {
	lockPath := filepath.Join(c.opts.Home, ".state", "agentsync.lock")
	lock, err := iox.AcquireLockTimeout(lockPath, 30*time.Second)
	if err != nil {
		return fmt.Errorf("acquire agentsync lock: %w", err)
	}
	defer func() { _ = lock.Release() }()
	return fn()
}

func (c *RuleCore) targetsStatePath() string {
	return filepath.Join(c.opts.Home, ".state", "targets.json")
}

func (c *RuleCore) projectRegistryPath() string {
	return filepath.Join(c.opts.Home, ".state", "agent-assistant", "projects.json")
}

func (c *RuleCore) recordWrittenRuleState(st *state.Targets, plan render.RenderPlan, written map[string]bool, sc adapter.Scope, projectPath string) {
	for agentName, result := range plan.PerAgent {
		var completed []adapter.FileOp
		for _, op := range memoryOps(result.Ops) {
			if written[op.Path] {
				completed = append(completed, op)
			}
		}
		if len(completed) > 0 {
			_ = render.RecordOpsState(st, c.opts.NativeRoot, agentName, sc, projectPath, completed)
		}
	}
	_ = state.Save(c.targetsStatePath(), st)
}

func memoryOps(ops []adapter.FileOp) []adapter.FileOp {
	out := make([]adapter.FileOp, 0, 1)
	for _, op := range ops {
		if op.SourceID == "memory/AGENTS.md" {
			out = append(out, op)
		}
	}
	return out
}

func memorySkips(skips []adapter.Skip) []adapter.Skip {
	var out []adapter.Skip
	for _, skip := range skips {
		if skip.Component == "memory" {
			out = append(out, skip)
		}
	}
	return out
}

func selectedAgents(cfg source.Config, requested []string) []string {
	seen := map[string]bool{}
	out := make([]string, 0, len(requested)+len(cfg.Agents))
	for _, name := range requested {
		name = strings.TrimSpace(name)
		if name != "" && !seen[name] {
			seen[name] = true
			out = append(out, name)
		}
	}
	if len(out) == 0 {
		for name, item := range cfg.Agents {
			if item.Enabled {
				out = append(out, name)
			}
		}
	}
	sort.Strings(out)
	return out
}

func ruleDocument(req RuleRequest, home, projectPath string, memory source.Memory, agents []string) RuleDocument {
	fragments := make([]string, 0, len(memory.Fragments))
	for name := range memory.Fragments {
		fragments = append(fragments, name)
	}
	sort.Strings(fragments)
	scope := req.Scope
	if scope == "" {
		scope = RuleScopeGlobal
	}
	return RuleDocument{
		Scope:         scope,
		ProjectPath:   projectPath,
		CanonicalPath: filepath.Join(home, "memory", "AGENTS.md"),
		Body:          memory.Body,
		Fragments:     fragments,
		Agents:        agents,
	}
}

func validateProjectRoot(path string) (string, error) {
	if strings.TrimSpace(path) == "" {
		return "", fmt.Errorf("project path is required")
	}
	abs, err := filepath.Abs(path)
	if err != nil {
		return "", fmt.Errorf("resolve project path: %w", err)
	}
	info, err := os.Stat(abs)
	if err != nil {
		return "", fmt.Errorf("inspect project path: %w", err)
	}
	if !info.IsDir() {
		return "", fmt.Errorf("project path %s is not a directory", abs)
	}
	return filepath.Clean(abs), nil
}

func sha256String(data []byte) string {
	sum := sha256.Sum256(data)
	return hex.EncodeToString(sum[:])
}

func ruleDiff(actual, desired string) string {
	if actual == desired {
		return ""
	}
	dmp := diffmatchpatch.New()
	diffs := dmp.DiffMain(actual, desired, false)
	var b strings.Builder
	for _, diff := range diffs {
		prefix := " "
		switch diff.Type {
		case diffmatchpatch.DiffDelete:
			prefix = "-"
		case diffmatchpatch.DiffInsert:
			prefix = "+"
		}
		for _, line := range strings.SplitAfter(diff.Text, "\n") {
			if line != "" {
				b.WriteString(prefix)
				b.WriteString(line)
			}
		}
	}
	return b.String()
}

func flattenRenderedRule(body string) string {
	var lines []string
	for _, line := range strings.Split(body, "\n") {
		trimmed := strings.TrimSpace(line)
		if strings.HasPrefix(trimmed, "<!-- agentsync:fragment ") || strings.HasPrefix(trimmed, "<!-- /agentsync:fragment ") {
			continue
		}
		lines = append(lines, line)
	}
	return strings.TrimSpace(strings.Join(lines, "\n")) + "\n"
}

func uniqueRuleBodies(sources []ProjectRuleSource) int {
	unique := map[string]bool{}
	for _, item := range sources {
		unique[strings.TrimSpace(item.Body)] = true
	}
	return len(unique)
}
