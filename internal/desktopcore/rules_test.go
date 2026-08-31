package desktopcore

import (
	"context"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/spxrogers/agentsync/internal/source"
	"github.com/spxrogers/agentsync/internal/testenv"
)

func TestRuleCoreSavePreservesFragmentsAndPreviewsTargets(t *testing.T) {
	testenv.RequireContainer(t)
	opts := ruleTestOptions(t)
	writeRuleTestConfig(t, opts.Home, "codex", "gemini")
	if err := source.WriteMemory(opts.Home, source.Memory{
		Body:      "# Global\n\n@import ./fragments/style.md\n",
		Fragments: map[string]string{"style.md": "Use existing styles.\n"},
	}); err != nil {
		t.Fatal(err)
	}

	core := NewRuleCore(opts, nil)
	doc, err := core.SaveRule(SaveRuleRequest{
		RuleRequest: RuleRequest{Scope: RuleScopeGlobal, Agents: []string{"codex", "gemini"}},
		Body:        "# Global\n\nPrefer current project patterns.\n\n@import ./fragments/style.md\n",
	})
	if err != nil {
		t.Fatal(err)
	}
	if doc.Body == "" || len(doc.Fragments) != 1 || doc.Fragments[0] != "style.md" {
		t.Fatalf("saved document lost its fragment structure: %+v", doc)
	}

	workspace, err := core.GetRules(RuleRequest{Scope: RuleScopeGlobal, Agents: []string{"codex", "gemini"}})
	if err != nil {
		t.Fatal(err)
	}
	if workspace.Blocked || len(workspace.Targets) != 2 {
		t.Fatalf("unexpected preview: %+v", workspace)
	}
	for _, target := range workspace.Targets {
		if target.Status != "new" || target.Blocked || target.Path == "" {
			t.Errorf("target should be a safe new file: %+v", target)
		}
	}
}

func TestRuleCoreSaveDoesNotRewriteUnchangedConfig(t *testing.T) {
	testenv.RequireContainer(t)
	opts := ruleTestOptions(t)
	if err := os.MkdirAll(opts.Home, 0o755); err != nil {
		t.Fatal(err)
	}
	configPath := filepath.Join(opts.Home, "agentsync.toml")
	wantConfig := "# Keep this project note.\n\n[agents.codex]\nenabled = true\n"
	if err := os.WriteFile(configPath, []byte(wantConfig), 0o644); err != nil {
		t.Fatal(err)
	}

	_, err := NewRuleCore(opts, nil).SaveRule(SaveRuleRequest{
		RuleRequest: RuleRequest{Scope: RuleScopeGlobal, Agents: []string{"codex"}},
		Body:        "# Updated Rule\n",
	})
	if err != nil {
		t.Fatal(err)
	}
	gotConfig, err := os.ReadFile(configPath)
	if err != nil {
		t.Fatal(err)
	}
	if string(gotConfig) != wantConfig {
		t.Fatalf("saving Rule rewrote an unchanged config:\n%s", gotConfig)
	}
}

func TestRuleCoreEmptyAgentSelectionSerializesAsAnArray(t *testing.T) {
	testenv.RequireContainer(t)
	core := NewRuleCore(ruleTestOptions(t), nil)
	workspace, err := core.GetRules(RuleRequest{Scope: RuleScopeGlobal})
	if err != nil {
		t.Fatal(err)
	}
	if workspace.Document.Agents == nil || workspace.Targets == nil || workspace.AvailableAgents == nil {
		t.Fatalf("desktop arrays must never serialize as null: %+v", workspace)
	}
}

func TestRuleCoreDoesNotTreatAnEmptyNativeFileAsAHandEdit(t *testing.T) {
	testenv.RequireContainer(t)
	opts := ruleTestOptions(t)
	writeRuleTestConfig(t, opts.Home, "codex")
	targetPath := filepath.Join(opts.NativeRoot, ".codex", "AGENTS.md")
	if err := os.MkdirAll(filepath.Dir(targetPath), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(targetPath, nil, 0o644); err != nil {
		t.Fatal(err)
	}
	workspace, err := NewRuleCore(opts, nil).GetRules(RuleRequest{Scope: RuleScopeGlobal, Agents: []string{"codex"}})
	if err != nil {
		t.Fatal(err)
	}
	if workspace.Blocked || workspace.Targets[0].Status != "empty" {
		t.Fatalf("an empty native file carries no Rule semantics and must not block: %+v", workspace)
	}
}

func TestRuleCoreSyncBlocksDriftThenBacksUpAndOverwrites(t *testing.T) {
	testenv.RequireContainer(t)
	opts := ruleTestOptions(t)
	writeRuleTestConfig(t, opts.Home, "codex")
	if err := source.WriteMemory(opts.Home, source.Memory{Body: "# Rule\n\nCanonical.\n"}); err != nil {
		t.Fatal(err)
	}
	core := NewRuleCore(opts, nil)
	req := SyncRulesRequest{RuleRequest: RuleRequest{Scope: RuleScopeGlobal, Agents: []string{"codex"}}}

	first, err := core.SyncRules(req)
	if err != nil {
		t.Fatal(err)
	}
	if !first.Applied || first.Preview.Blocked {
		t.Fatalf("first sync did not apply: %+v", first)
	}
	targetPath := first.Preview.Targets[0].Path
	if err := os.WriteFile(targetPath, []byte("# Hand edit\n\nKeep this.\n"), 0o644); err != nil {
		t.Fatal(err)
	}

	blocked, err := core.SyncRules(req)
	if err != nil {
		t.Fatal(err)
	}
	if blocked.Applied || !blocked.Preview.Blocked || blocked.Preview.Targets[0].Diff == "" {
		t.Fatalf("native drift was not blocked with a diff: %+v", blocked)
	}
	got, err := os.ReadFile(targetPath)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(got), "Keep this") {
		t.Fatal("blocked sync overwrote the hand edit")
	}

	forced, err := core.SyncRules(SyncRulesRequest{
		RuleRequest: req.RuleRequest,
		Resolution:  RuleResolutionBackupOverwrite,
	})
	if err != nil {
		t.Fatal(err)
	}
	if !forced.Applied || len(forced.Backups) != 1 {
		t.Fatalf("backup-overwrite did not report its backup: %+v", forced)
	}
	backup, err := os.ReadFile(forced.Backups[0].BackupPath)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(backup), "Keep this") {
		t.Fatal("backup did not preserve the native edit")
	}
	got, err = os.ReadFile(targetPath)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(got), "Canonical") || strings.Contains(string(got), "Keep this") {
		t.Fatalf("destination did not converge to canonical: %s", got)
	}
}

func TestRuleCoreImportsOneNativeRuleIntoMother(t *testing.T) {
	testenv.RequireContainer(t)
	opts := ruleTestOptions(t)
	writeRuleTestConfig(t, opts.Home, "codex")
	core := NewRuleCore(opts, nil)
	workspace, err := core.GetRules(RuleRequest{Scope: RuleScopeGlobal, Agents: []string{"codex"}})
	if err != nil {
		t.Fatal(err)
	}
	targetPath := workspace.Targets[0].Path
	if targetPath == "" {
		t.Fatal("empty mother template must still expose the verified native Rule path")
	}
	if err := os.MkdirAll(filepath.Dir(targetPath), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(targetPath, []byte("# Imported\n\nNative decision.\n"), 0o644); err != nil {
		t.Fatal(err)
	}

	doc, err := core.ImportNativeRule(ImportNativeRuleRequest{
		RuleRequest: RuleRequest{Scope: RuleScopeGlobal, Agents: []string{"codex"}},
		Agent:       "codex",
	})
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(doc.Body, "Native decision") {
		t.Fatalf("native rule was not imported: %+v", doc)
	}
}

func TestRuleCoreRegistersProjectOutsideProjectGitAndFindsNativeRules(t *testing.T) {
	testenv.RequireContainer(t)
	opts := ruleTestOptions(t)
	projectRoot := filepath.Join(t.TempDir(), "existing-project")
	if err := os.MkdirAll(projectRoot, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(projectRoot, "AGENTS.md"), []byte("# Existing project rule\n"), 0o644); err != nil {
		t.Fatal(err)
	}

	core := NewRuleCore(opts, nil)
	result, err := core.ImportProject(projectRoot)
	if err != nil {
		t.Fatal(err)
	}
	if result.Path != projectRoot || len(result.Sources) == 0 {
		t.Fatalf("project import missed native rules: %+v", result)
	}
	if _, err := os.Stat(filepath.Join(projectRoot, ".agentsync")); !os.IsNotExist(err) {
		t.Fatalf("project import must not initialize project files, stat err=%v", err)
	}
	registryPath := filepath.Join(opts.Home, ".state", "agent-assistant", "projects.json")
	if _, err := os.Stat(registryPath); err != nil {
		t.Fatalf("local project registry was not written: %v", err)
	}
}

func TestRuleCoreAnalyzesProjectRulesWithoutWritingCandidate(t *testing.T) {
	testenv.RequireContainer(t)
	opts := ruleTestOptions(t)
	projectRoot := filepath.Join(t.TempDir(), "mixed-project")
	if err := os.MkdirAll(filepath.Join(projectRoot, ".claude"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(projectRoot, "AGENTS.md"), []byte("# Shared\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(projectRoot, "CLAUDE.md"), []byte("# Claude\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	analyzer := &stubRuleAnalyzer{proposal: RuleProposal{Body: "# Unified\n", Notes: []string{"Kept both policies."}, Analyzer: "test-ai"}}
	core := NewRuleCore(opts, analyzer)
	if _, err := core.ImportProject(projectRoot); err != nil {
		t.Fatal(err)
	}

	proposal, err := core.AnalyzeProjectRules(context.Background(), projectRoot)
	if err != nil {
		t.Fatal(err)
	}
	if proposal.Body != "# Unified\n" || analyzer.calls != 1 {
		t.Fatalf("AI proposal was not returned: proposal=%+v calls=%d", proposal, analyzer.calls)
	}
	if _, err := os.Stat(filepath.Join(projectRoot, ".agentsync", "memory", "AGENTS.md")); !os.IsNotExist(err) {
		t.Fatalf("analysis must not save its candidate, stat err=%v", err)
	}
}

type stubRuleAnalyzer struct {
	proposal RuleProposal
	calls    int
}

func (s *stubRuleAnalyzer) Analyze(_ context.Context, _ string, _ []ProjectRuleSource) (RuleProposal, error) {
	s.calls++
	return s.proposal, nil
}

func ruleTestOptions(t *testing.T) Options {
	t.Helper()
	root := t.TempDir()
	return Options{
		Home:         filepath.Join(root, "agentsync-home"),
		ProjectsRoot: filepath.Join(root, "projects"),
		NativeRoot:   filepath.Join(root, "user-home"),
	}
}

func writeRuleTestConfig(t *testing.T, home string, agents ...string) {
	t.Helper()
	if err := os.MkdirAll(home, 0o755); err != nil {
		t.Fatal(err)
	}
	var body strings.Builder
	body.WriteString("[agents]\n")
	for _, agent := range agents {
		body.WriteString(agent + " = { enabled = true }\n")
	}
	if err := os.WriteFile(filepath.Join(home, "agentsync.toml"), []byte(body.String()), 0o644); err != nil {
		t.Fatal(err)
	}
}
