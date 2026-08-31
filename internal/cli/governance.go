package cli

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"

	"github.com/spf13/afero"
	"github.com/spf13/cobra"
	"github.com/spxrogers/agentsync/internal/governance"
	"github.com/spxrogers/agentsync/internal/project"
	"github.com/spxrogers/agentsync/internal/source"
)

// governanceScan is deliberately small and read-only. It gives an Agent the
// evidence needed to select a Profile before any project file is changed.
type governanceScan struct {
	ProjectRoot       string   `json:"projectRoot"`
	ProfileID         string   `json:"profileId"`
	ProfileFound      bool     `json:"profileFound"`
	PackageManager    string   `json:"packageManager"`
	Lockfiles         []string `json:"lockfiles"`
	Dependencies      []string `json:"dependencies"`
	ExistingAdapters  []string `json:"existingAdapters"`
	DocumentationRoot string   `json:"documentationRoot"`
	DocumentationOK   bool     `json:"documentationAvailable"`
	Notes             []string `json:"notes,omitempty"`
}

type governanceVerify struct {
	OK           bool                                   `json:"ok"`
	ProjectRoot  string                                 `json:"projectRoot"`
	Profile      string                                 `json:"profile"`
	Issues       []string                               `json:"issues,omitempty"`
	Capabilities map[string]governance.CapabilityReport `json:"capabilities,omitempty"`
}

func newGovernanceCmd() *cobra.Command {
	cmd := strictGroup(&cobra.Command{
		Use:   "governance",
		Short: "preflight and verify project technology governance",
		Long: "Inspect a project's existing stack, initialize a local governance " +
			"snapshot, and verify the Baseline/Profile contract before an Agent edits code.",
	})
	markScopeAware(cmd)
	cmd.AddCommand(newGovernanceScanCmd(), newGovernanceInitCmd(), newGovernanceVerifyCmd(), newGovernanceCapabilitiesCmd())
	return cmd
}

func newGovernanceScanCmd() *cobra.Command {
	var profileID string
	var jsonOutput bool
	cmd := &cobra.Command{
		Use:   "scan",
		Short: "inspect a project without changing files",
		Args:  cobra.NoArgs,
		RunE: func(cmd *cobra.Command, _ []string) error {
			root, err := governanceProjectRoot(cmd)
			if err != nil {
				return err
			}
			result, err := scanGovernanceProject(root, profileID)
			if err != nil {
				return err
			}
			if jsonOutput {
				return emitJSON(cmd.OutOrStdout(), result)
			}
			p, err := newPrinter(cmd)
			if err != nil {
				return err
			}
			fmt.Fprintf(p.Out, "Project: %s\nProfile: %s (%s)\nPackage manager: %s\n", result.ProjectRoot, result.ProfileID, profileStatus(result.ProfileFound), result.PackageManager)
			fmt.Fprintf(p.Out, "Lockfiles: %s\nDependencies: %d\nExisting adapters: %s\n", strings.Join(result.Lockfiles, ", "), len(result.Dependencies), strings.Join(result.ExistingAdapters, ", "))
			if result.DocumentationRoot != "" {
				fmt.Fprintf(p.Out, "Documentation: %s (%s)\n", result.DocumentationRoot, boolStatus(result.DocumentationOK))
			}
			for _, note := range result.Notes {
				fmt.Fprintf(p.Out, "Note: %s\n", note)
			}
			return nil
		},
	}
	cmd.Flags().StringVar(&profileID, "profile", "", "governance Profile id (default: infer from project directory)")
	cmd.Flags().BoolVar(&jsonOutput, "json", false, "emit machine-readable JSON")
	markScopeAware(cmd)
	return cmd
}

func newGovernanceInitCmd() *cobra.Command {
	var profileID string
	var agents string
	var dryRun bool
	var force bool
	cmd := &cobra.Command{
		Use:   "init",
		Short: "initialize local governance state for a project",
		Args:  cobra.NoArgs,
		RunE: func(cmd *cobra.Command, _ []string) error {
			root, err := governanceProjectRoot(cmd)
			if err != nil {
				return err
			}
			profile, err := selectGovernanceProfile(root, profileID)
			if err != nil {
				return err
			}
			agentNames, err := governanceAgents(profile, agents)
			if err != nil {
				return err
			}
			manifest := governance.BuildManifest(profile, root, agentNames)
			plan := governanceInitPlan(root, profile, manifest)
			if dryRun {
				p, perr := newPrinter(cmd)
				if perr != nil {
					return perr
				}
				fmt.Fprintln(p.Out, "Governance initialization plan (dry-run):")
				for _, path := range plan.paths {
					fmt.Fprintf(p.Out, "  %s\n", path)
				}
				fmt.Fprintf(p.Out, "Profile: %s\nAgents: %s\nNo files changed.\n", profile.ID, strings.Join(agentNames, ", "))
				return nil
			}
			if err := plan.apply(force); err != nil {
				return err
			}
			p, perr := newPrinter(cmd)
			if perr != nil {
				return perr
			}
			p.Successf("✅", "initialized governance Profile %s for %s", profile.ID, root)
			fmt.Fprintf(p.Out, "Generated state is local-only; run `agentsync apply --scope project --project %s` to render native adapters.\n", root)
			return nil
		},
	}
	cmd.Flags().StringVar(&profileID, "profile", "", "governance Profile id (default: infer from project directory)")
	cmd.Flags().StringVar(&agents, "agents", "", "comma-separated agent adapters (default: Profile agents or codex,cursor,gemini,antigravity)")
	cmd.Flags().BoolVar(&dryRun, "dry-run", false, "show generated paths without writing")
	cmd.Flags().BoolVar(&force, "force", false, "replace governance-generated files, never existing native adapter files")
	markScopeAware(cmd)
	return cmd
}

func newGovernanceVerifyCmd() *cobra.Command {
	var jsonOutput bool
	cmd := &cobra.Command{
		Use:   "check",
		Short: "verify local governance state and capability loss",
		Args:  cobra.NoArgs,
		RunE: func(cmd *cobra.Command, _ []string) error {
			root, err := governanceProjectRoot(cmd)
			if err != nil {
				return err
			}
			result := verifyGovernanceProject(root)
			if jsonOutput {
				if err := emitJSON(cmd.OutOrStdout(), result); err != nil {
					return err
				}
				if !result.OK {
					return errors.New("governance verification failed")
				}
				return nil
			}
			p, perr := newPrinter(cmd)
			if perr != nil {
				return perr
			}
			if result.OK {
				p.Successf("✅", "governance verified for %s (Profile %s)", root, result.Profile)
			} else {
				for _, issue := range result.Issues {
					p.Errorf("governance: %s", issue)
				}
				return errors.New("governance verification failed")
			}
			for agent, report := range result.Capabilities {
				fmt.Fprintf(p.Out, "  %s: %s\n", agent, capabilitySummary(report))
			}
			return nil
		},
	}
	cmd.Flags().BoolVar(&jsonOutput, "json", false, "emit machine-readable JSON")
	markScopeAware(cmd)
	return cmd
}

func newGovernanceCapabilitiesCmd() *cobra.Command {
	var agent string
	var jsonOutput bool
	cmd := &cobra.Command{
		Use:   "capabilities",
		Short: "show the explicit cross-agent capability matrix",
		Args:  cobra.NoArgs,
		RunE: func(cmd *cobra.Command, _ []string) error {
			names := []string{"antigravity", "codex", "cursor", "gemini"}
			if agent != "" {
				names = []string{agent}
			}
			out := make(map[string]governance.CapabilityReport, len(names))
			for _, name := range names {
				report, ok := governance.Capabilities(name)
				if !ok {
					return fmt.Errorf("unknown capability agent %q", name)
				}
				out[name] = report
			}
			if jsonOutput {
				return emitJSON(cmd.OutOrStdout(), out)
			}
			p, err := newPrinter(cmd)
			if err != nil {
				return err
			}
			for _, name := range names {
				fmt.Fprintf(p.Out, "%s: %s\n", name, capabilitySummary(out[name]))
			}
			return nil
		},
	}
	cmd.Flags().StringVar(&agent, "agent", "", "show one agent capability report")
	cmd.Flags().BoolVar(&jsonOutput, "json", false, "emit machine-readable JSON")
	markScopeUnaware(cmd, "capabilities is a machine-independent registry report")
	return cmd
}

func governanceProjectRoot(cmd *cobra.Command) (string, error) {
	_, projectFlag := scopeFlagValues(cmd)
	root := projectFlag
	if root == "" {
		var err error
		root, err = os.Getwd()
		if err != nil {
			return "", fmt.Errorf("resolve project root: %w", err)
		}
	}
	abs, err := filepath.Abs(root)
	if err != nil {
		return "", fmt.Errorf("resolve project root %q: %w", root, err)
	}
	info, err := os.Stat(abs)
	if err != nil || !info.IsDir() {
		return "", fmt.Errorf("project root %q is not a directory", abs)
	}
	return abs, nil
}

func selectGovernanceProfile(root, requested string) (governance.ProfileData, error) {
	if requested != "" {
		if profile, ok := governance.Profile(requested); ok {
			return profile, nil
		}
		return governance.ProfileData{}, fmt.Errorf("unknown governance Profile %q", requested)
	}
	base := filepath.Base(root)
	if profile, ok := governance.Profile(base); ok {
		return profile, nil
	}
	return governance.ProfileData{}, fmt.Errorf("cannot infer a governance Profile for %q; pass --profile", base)
}

func governanceAgents(profile governance.ProfileData, requested string) ([]string, error) {
	if requested == "" {
		if len(profile.MaterializationAgents) > 0 {
			return append([]string(nil), profile.MaterializationAgents...), nil
		}
		return []string{"codex", "cursor", "gemini", "antigravity"}, nil
	}
	var result []string
	seen := map[string]bool{}
	for _, item := range strings.Split(requested, ",") {
		name := strings.TrimSpace(item)
		if name == "" || seen[name] {
			continue
		}
		if _, ok := governance.Capabilities(name); !ok {
			return nil, fmt.Errorf("unknown governance agent %q", name)
		}
		seen[name] = true
		result = append(result, name)
	}
	if len(result) == 0 {
		return nil, errors.New("--agents must contain at least one agent")
	}
	return result, nil
}

func scanGovernanceProject(root, requested string) (governanceScan, error) {
	profile, profileErr := selectGovernanceProfile(root, requested)
	result := governanceScan{ProjectRoot: root, ProfileFound: profileErr == nil}
	if profileErr == nil {
		result.ProfileID = profile.ID
		result.PackageManager = profile.PackageManager
		result.DocumentationRoot = profile.DocumentationRoot
		if _, err := os.Stat(profile.DocumentationRoot); err == nil {
			result.DocumentationOK = true
		}
	} else if requested != "" {
		return result, profileErr
	}
	for _, name := range []string{"package-lock.json", "yarn.lock", "pnpm-lock.yaml", "bun.lockb", "bun.lock"} {
		if _, err := os.Stat(filepath.Join(root, name)); err == nil {
			result.Lockfiles = append(result.Lockfiles, name)
		}
	}
	if len(result.Lockfiles) == 0 {
		result.Notes = append(result.Notes, "no JavaScript lockfile found")
	}
	if data, err := os.ReadFile(filepath.Join(root, "package.json")); err == nil {
		var pkg struct {
			Dependencies    map[string]json.RawMessage `json:"dependencies"`
			DevDependencies map[string]json.RawMessage `json:"devDependencies"`
		}
		if err := json.Unmarshal(data, &pkg); err != nil {
			return result, fmt.Errorf("parse package.json: %w", err)
		}
		for name := range pkg.Dependencies {
			result.Dependencies = append(result.Dependencies, name)
		}
		for name := range pkg.DevDependencies {
			result.Dependencies = append(result.Dependencies, name)
		}
		sort.Strings(result.Dependencies)
	} else {
		result.Notes = append(result.Notes, "package.json not found; select a Profile explicitly")
	}
	for _, name := range []string{"AGENTS.md", "CLAUDE.md", "GEMINI.md", ".cursor", ".gemini", ".agents"} {
		if _, err := os.Stat(filepath.Join(root, name)); err == nil {
			result.ExistingAdapters = append(result.ExistingAdapters, name)
		}
	}
	return result, nil
}

type governanceInit struct {
	root     string
	profile  governance.ProfileData
	manifest governance.Manifest
	paths    []string
}

func governanceInitPlan(root string, profile governance.ProfileData, manifest governance.Manifest) governanceInit {
	return governanceInit{root: root, profile: profile, manifest: manifest, paths: []string{
		filepath.Join(root, ".agentsync", "agentsync.toml"),
		filepath.Join(root, ".agentsync", "memory", "AGENTS.md"),
		filepath.Join(root, ".agentsync", "README.md"),
		filepath.Join(root, ".agent-governance", "manifest.json"),
		".git/info/exclude (append local-only generated paths)",
	}}
}

func (plan governanceInit) apply(force bool) error {
	configPath := filepath.Join(plan.root, ".agentsync", "agentsync.toml")
	memoryPath := filepath.Join(plan.root, ".agentsync", "memory", "AGENTS.md")
	readmePath := filepath.Join(plan.root, ".agentsync", "README.md")
	manifestPath := filepath.Join(plan.root, ".agent-governance", "manifest.json")
	for _, path := range []string{configPath, memoryPath, readmePath, manifestPath} {
		if _, err := os.Stat(path); err == nil && !force {
			return fmt.Errorf("refusing to overwrite %s; use --force only for governance-generated files", path)
		} else if err != nil && !os.IsNotExist(err) {
			return fmt.Errorf("inspect %s: %w", path, err)
		}
	}
	if err := os.MkdirAll(filepath.Dir(memoryPath), 0o755); err != nil {
		return err
	}
	if err := os.MkdirAll(filepath.Dir(manifestPath), 0o755); err != nil {
		return err
	}
	if err := os.WriteFile(configPath, []byte(renderGovernanceConfig(plan.manifest.Agents)), 0o644); err != nil {
		return fmt.Errorf("write %s: %w", configPath, err)
	}
	if err := os.WriteFile(memoryPath, []byte(governance.RenderMemory(plan.profile, plan.manifest.BaselineSHA256)), 0o644); err != nil {
		return fmt.Errorf("write %s: %w", memoryPath, err)
	}
	readme := "# Local Agent Governance\n\nThis `.agentsync/` tree is generated by `agentsync governance init`. It is local runtime state.\nThe canonical Baseline/Profile are embedded in the governance materializer; do not add secrets or fonts.\nRun `agentsync governance check --scope project` before editing and `agentsync apply --scope project --dry-run` to inspect adapters.\n"
	if err := os.WriteFile(readmePath, []byte(readme), 0o644); err != nil {
		return fmt.Errorf("write %s: %w", readmePath, err)
	}
	manifestBytes, err := json.MarshalIndent(plan.manifest, "", "  ")
	if err != nil {
		return err
	}
	manifestBytes = append(manifestBytes, '\n')
	if err := os.WriteFile(manifestPath, manifestBytes, 0o644); err != nil {
		return fmt.Errorf("write %s: %w", manifestPath, err)
	}
	return appendGovernanceExcludes(plan.root, governance.LocalIgnoreEntries(plan.profile))
}

func renderGovernanceConfig(agents []string) string {
	var b strings.Builder
	b.WriteString("# Generated local governance project source; do not put secrets here.\n\n[agents]\n")
	for _, name := range agents {
		fmt.Fprintf(&b, "%s = { enabled = true }\n", name)
	}
	return b.String()
}

func appendGovernanceExcludes(root string, entries []string) error {
	gitDirCmd := exec.Command("git", "-C", root, "rev-parse", "--git-path", "info/exclude")
	out, err := gitDirCmd.Output()
	if err != nil {
		return fmt.Errorf("resolve Git info/exclude: %w", err)
	}
	excludePath := strings.TrimSpace(string(out))
	if !filepath.IsAbs(excludePath) {
		excludePath = filepath.Join(root, excludePath)
	}
	data, err := os.ReadFile(excludePath)
	if err != nil && !os.IsNotExist(err) {
		return fmt.Errorf("read %s: %w", excludePath, err)
	}
	text := string(data)
	for _, entry := range entries {
		if !containsLine(text, entry) {
			text += fmt.Sprintf("\n# agentsync governance local-only state\n%s\n", entry)
		}
	}
	if err := os.MkdirAll(filepath.Dir(excludePath), 0o755); err != nil {
		return err
	}
	return os.WriteFile(excludePath, []byte(text), 0o644)
}

func verifyGovernanceProject(root string) governanceVerify {
	result := governanceVerify{OK: true, ProjectRoot: root, Capabilities: map[string]governance.CapabilityReport{}}
	manifestPath := filepath.Join(root, ".agent-governance", "manifest.json")
	data, err := os.ReadFile(manifestPath)
	if err != nil {
		result.Issues = append(result.Issues, fmt.Sprintf("read manifest: %v", err))
		result.OK = false
		return result
	}
	var manifest governance.Manifest
	if err := json.Unmarshal(data, &manifest); err != nil {
		result.Issues = append(result.Issues, fmt.Sprintf("parse manifest: %v", err))
		result.OK = false
		return result
	}
	result.Profile = manifest.ProfileID
	profile, ok := governance.Profile(manifest.ProfileID)
	if !ok {
		result.Issues = append(result.Issues, "manifest references an unknown Profile")
		result.OK = false
		return result
	}
	if manifest.ProjectRoot != root {
		result.Issues = append(result.Issues, fmt.Sprintf("manifest projectRoot %q does not match %q", manifest.ProjectRoot, root))
	}
	if manifest.BaselineSHA256 != governance.BaselineSHA256() {
		result.Issues = append(result.Issues, "Baseline hash differs from the embedded governance source")
	}
	profileHash, _ := governance.ProfileSHA256(profile.ID)
	if manifest.ProfileSHA256 != profileHash {
		result.Issues = append(result.Issues, "Profile hash differs from the embedded governance source")
	}
	if _, err := os.Stat(filepath.Join(root, ".agentsync", "agentsync.toml")); err != nil {
		result.Issues = append(result.Issues, "project .agentsync/agentsync.toml is missing")
	}
	if memory, err := os.ReadFile(filepath.Join(root, ".agentsync", "memory", "AGENTS.md")); err != nil {
		result.Issues = append(result.Issues, "project governance memory is missing")
	} else if !strings.Contains(string(memory), manifest.BaselineSHA256) {
		result.Issues = append(result.Issues, "rendered governance memory has no matching Baseline provenance")
	}
	if profile.DocumentationRoot != "" {
		if _, err := os.Stat(profile.DocumentationRoot); err != nil {
			result.Issues = append(result.Issues, fmt.Sprintf("documentation root unavailable: %s", profile.DocumentationRoot))
		}
	}
	if _, err := source.Load(afero.NewOsFs(), project.Home(root)); err != nil {
		result.Issues = append(result.Issues, fmt.Sprintf("project source is invalid: %v", err))
	}
	for _, name := range manifest.Agents {
		if report, ok := governance.Capabilities(name); ok {
			result.Capabilities[name] = report
		} else {
			result.Issues = append(result.Issues, fmt.Sprintf("manifest references unknown agent %q", name))
		}
	}
	result.OK = len(result.Issues) == 0
	return result
}

func capabilitySummary(report governance.CapabilityReport) string {
	parts := make([]string, 0, len(report.Components))
	for _, component := range []string{"rules", "skills", "mcp", "commands", "hooks", "subagents"} {
		if state, ok := report.Components[component]; ok {
			parts = append(parts, component+"="+string(state))
		}
	}
	return strings.Join(parts, ", ")
}

func profileStatus(found bool) string {
	if found {
		return "found"
	}
	return "not found"
}

func boolStatus(value bool) string {
	if value {
		return "available"
	}
	return "missing"
}

func containsLine(text, wanted string) bool {
	for _, line := range strings.Split(text, "\n") {
		if strings.TrimSpace(line) == wanted {
			return true
		}
	}
	return false
}
