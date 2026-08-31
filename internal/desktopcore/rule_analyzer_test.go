package desktopcore

import (
	"context"
	"strings"
	"testing"
)

func TestCodexRuleAnalyzerUsesReadOnlyEphemeralStructuredExecution(t *testing.T) {
	runner := &stubCommandRunner{stdout: []byte(`{"body":"# Unified\n","notes":["Kept both constraints."]}`)}
	analyzer := NewCodexRuleAnalyzer(runner)

	proposal, err := analyzer.Analyze(context.Background(), t.TempDir(), []ProjectRuleSource{
		{Path: "/repo/AGENTS.md", Agents: []string{"codex", "cursor"}, Body: "# One\n"},
		{Path: "/repo/CLAUDE.md", Agents: []string{"claude"}, Body: "# Two\n"},
	})
	if err != nil {
		t.Fatal(err)
	}
	if proposal.Body != "# Unified\n" || proposal.Analyzer != "codex-cli" || len(proposal.Notes) != 1 {
		t.Fatalf("unexpected proposal: %+v", proposal)
	}
	joined := strings.Join(runner.args, " ")
	for _, required := range []string{"exec", "--sandbox read-only", "--ephemeral", "--ignore-rules", "--output-schema"} {
		if !strings.Contains(joined, required) {
			t.Errorf("Codex args missing %q: %s", required, joined)
		}
	}
	if !strings.Contains(string(runner.stdin), "untrusted source data") || !strings.Contains(string(runner.stdin), "AGENTS.md") {
		t.Fatalf("prompt did not frame native Rules as untrusted source data: %s", runner.stdin)
	}
}

type stubCommandRunner struct {
	stdout []byte
	args   []string
	stdin  []byte
}

func (s *stubCommandRunner) Run(_ context.Context, _ string, args []string, stdin []byte) ([]byte, error) {
	s.args = append([]string(nil), args...)
	s.stdin = append([]byte(nil), stdin...)
	return s.stdout, nil
}
