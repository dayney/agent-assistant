package governance_test

import (
	"strings"
	"testing"

	"github.com/spxrogers/agentsync/internal/governance"
)

func TestEmbeddedProfilesDeclareStackCommandsAndFontPolicy(t *testing.T) {
	profiles := governance.Profiles()
	if len(profiles) < 3 {
		t.Fatalf("Profiles() = %d profiles, want the three supported project profiles", len(profiles))
	}
	for _, profile := range profiles {
		if profile.ID == "" || len(profile.Stack) == 0 || len(profile.Commands) == 0 {
			t.Fatalf("profile %q must declare id, stack, and verification commands: %+v", profile.ID, profile)
		}
		if profile.FontPolicy == "" || !strings.Contains(strings.ToLower(profile.FontPolicy), "no") {
			t.Fatalf("profile %q must state its no-new-font policy: %q", profile.ID, profile.FontPolicy)
		}
	}
}

func TestBaselineContainsPreflightStopAndAskAndFontBoundary(t *testing.T) {
	baseline := governance.Baseline()
	for _, required := range []string{
		"Before Any Edit",
		"Stop-and-Ask Boundary",
		"font-[Poppins]",
		"explicit approval",
		"full",
		"partial",
		"unsupported",
	} {
		if !strings.Contains(baseline, required) {
			t.Errorf("Baseline() does not contain %q", required)
		}
	}
}

func TestRenderMemoryKeepsBaselineAndProfileProvenance(t *testing.T) {
	profile, ok := governance.Profile("songai")
	if !ok {
		t.Fatal("songai profile is not registered")
	}
	memory := governance.RenderMemory(profile, "baseline-sha256")
	for _, required := range []string{
		"baseline-sha256",
		"songai",
		"Next.js 15",
		"next-intl",
	} {
		if !strings.Contains(memory, required) {
			t.Errorf("rendered memory does not contain %q", required)
		}
	}
}

func TestCapabilitiesAreExplicitForAllManagedAgents(t *testing.T) {
	for _, agent := range []string{"codex", "cursor", "gemini", "antigravity"} {
		report, ok := governance.Capabilities(agent)
		if !ok {
			t.Fatalf("missing capability report for %s", agent)
		}
		for _, component := range []string{"rules", "skills", "mcp", "commands", "hooks", "subagents"} {
			state := report.Components[component]
			if state != governance.Full && state != governance.Partial && state != governance.Unsupported {
				t.Errorf("%s/%s has implicit capability state %q", agent, component, state)
			}
		}
	}
}

func TestLocalIgnoreEntriesDoNotIncludeCanonicalPolicySources(t *testing.T) {
	entries := governance.LocalIgnoreEntries(governance.ProfileData{LocalAdapters: []string{"AGENTS.md"}})
	joined := strings.Join(entries, "\n")
	for _, required := range []string{".agent-governance/", ".agentsync/", "AGENTS.md"} {
		if !strings.Contains(joined, required) {
			t.Errorf("local ignore entries missing %q: %v", required, entries)
		}
	}
	for _, forbidden := range []string{"CLAUDE.md", "GEMINI.md"} {
		if strings.Contains(joined, forbidden) {
			t.Errorf("local ignore entries must not hide existing adapter %q: %v", forbidden, entries)
		}
	}
}
