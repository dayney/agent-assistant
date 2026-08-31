// Package governance contains the cross-project coding-agent preflight model.
// It is intentionally independent from a project's business rules: the
// embedded Baseline defines universal safety/decision boundaries, while each
// Profile supplies that project's existing stack and verification commands.
package governance

import (
	"bytes"
	"crypto/sha256"
	"embed"
	"encoding/json"
	"fmt"
	"sort"
	"strings"

	"github.com/pelletier/go-toml/v2"
)

//go:embed assets/baseline.md assets/profiles/*.toml assets/capabilities.json
var assets embed.FS

// ProfileData is a project-specific technology and workflow contract.
type ProfileData struct {
	ID                    string            `toml:"id" json:"id"`
	Version               string            `toml:"version" json:"version"`
	ProjectRoot           string            `toml:"project_root" json:"projectRoot"`
	DocumentationRoot     string            `toml:"documentation_root" json:"documentationRoot"`
	SourceDocuments       []string          `toml:"source_documents" json:"sourceDocuments"`
	PackageManager        string            `toml:"package_manager" json:"packageManager"`
	MaterializationAgents []string          `toml:"materialization_agents" json:"materializationAgents"`
	LocalAdapters         []string          `toml:"local_adapters" json:"localAdapters"`
	Stack                 []string          `toml:"stack" json:"stack"`
	Commands              map[string]string `toml:"commands" json:"commands"`
	Defaults              map[string]string `toml:"defaults" json:"defaults"`
	Rules                 []string          `toml:"rules" json:"rules"`
	FontPolicy            string            `toml:"font_policy" json:"fontPolicy"`
}

// CapabilityState is the adapter fidelity state for one managed component.
type CapabilityState string

const (
	Full        CapabilityState = "full"
	Partial     CapabilityState = "partial"
	Unsupported CapabilityState = "unsupported"
)

// CapabilityReport describes what a target agent can receive from the
// canonical source. Partial and unsupported states must be surfaced to users.
type CapabilityReport struct {
	Tier       string                     `json:"tier"`
	Summary    string                     `json:"summary"`
	Components map[string]CapabilityState `json:"components"`
}

// Manifest records the immutable inputs and local-only outputs of a project
// governance initialization. It is safe to commit only when the project wants
// to share this metadata; generated adapter files themselves are never copied
// into the manifest.
type Manifest struct {
	SchemaVersion      int      `json:"schemaVersion"`
	BaselineSHA256     string   `json:"baselineSha256"`
	ProfileID          string   `json:"profileId"`
	ProfileVersion     string   `json:"profileVersion"`
	ProfileSHA256      string   `json:"profileSha256"`
	ProjectRoot        string   `json:"projectRoot"`
	DocumentationRoot  string   `json:"documentationRoot"`
	PackageManager     string   `json:"packageManager"`
	Agents             []string `json:"agents"`
	GeneratedPaths     []string `json:"generatedPaths"`
	SourceDocuments    []string `json:"sourceDocuments"`
	CapabilityRegistry string   `json:"capabilityRegistry"`
}

// BuildManifest creates a provenance record for a project initialization.
func BuildManifest(profile ProfileData, projectRoot string, agents []string) Manifest {
	profileHash, _ := ProfileSHA256(profile.ID)
	return Manifest{
		SchemaVersion:      1,
		BaselineSHA256:     BaselineSHA256(),
		ProfileID:          profile.ID,
		ProfileVersion:     profile.Version,
		ProfileSHA256:      profileHash,
		ProjectRoot:        projectRoot,
		DocumentationRoot:  profile.DocumentationRoot,
		PackageManager:     profile.PackageManager,
		Agents:             append([]string(nil), agents...),
		GeneratedPaths:     LocalIgnoreEntries(profile),
		SourceDocuments:    append([]string(nil), profile.SourceDocuments...),
		CapabilityRegistry: "embedded://governance/capabilities.json",
	}
}

// Baseline returns the technology-neutral preflight contract.
func Baseline() string {
	data, err := assets.ReadFile("assets/baseline.md")
	if err != nil {
		panic(fmt.Sprintf("embedded governance baseline missing: %v", err))
	}
	return string(data)
}

// BaselineSHA256 identifies the exact embedded Baseline used for a render.
func BaselineSHA256() string {
	return sha256Hex([]byte(Baseline()))
}

// Profiles returns all bundled project Profiles sorted by id.
func Profiles() []ProfileData {
	entries, err := assets.ReadDir("assets/profiles")
	if err != nil {
		panic(fmt.Sprintf("embedded governance profiles missing: %v", err))
	}
	profiles := make([]ProfileData, 0, len(entries))
	for _, entry := range entries {
		if entry.IsDir() || !strings.HasSuffix(entry.Name(), ".toml") {
			continue
		}
		profile, err := loadProfile(entry.Name())
		if err != nil {
			panic(err)
		}
		profiles = append(profiles, profile)
	}
	sort.Slice(profiles, func(i, j int) bool { return profiles[i].ID < profiles[j].ID })
	return profiles
}

// Profile returns a bundled Profile by id.
func Profile(id string) (ProfileData, bool) {
	for _, profile := range Profiles() {
		if profile.ID == id {
			return profile, true
		}
	}
	return ProfileData{}, false
}

// ProfileSHA256 identifies the exact embedded Profile source used for a render.
func ProfileSHA256(id string) (string, bool) {
	data, err := assets.ReadFile("assets/profiles/" + id + ".toml")
	if err != nil {
		return "", false
	}
	return sha256Hex(data), true
}

func sha256Hex(data []byte) string {
	sum := sha256.Sum256(data)
	return fmt.Sprintf("%x", sum[:])
}

func loadProfile(filename string) (ProfileData, error) {
	data, err := assets.ReadFile("assets/profiles/" + filename)
	if err != nil {
		return ProfileData{}, fmt.Errorf("read embedded profile %s: %w", filename, err)
	}
	var profile ProfileData
	decoder := toml.NewDecoder(bytes.NewReader(data))
	decoder.DisallowUnknownFields()
	if err := decoder.Decode(&profile); err != nil {
		return ProfileData{}, fmt.Errorf("parse embedded profile %s: %w", filename, err)
	}
	if err := ValidateProfile(profile); err != nil {
		return ProfileData{}, fmt.Errorf("invalid embedded profile %s: %w", filename, err)
	}
	return profile, nil
}

// ValidateProfile rejects incomplete Profiles before they can guide an Agent.
func ValidateProfile(profile ProfileData) error {
	if strings.TrimSpace(profile.ID) == "" {
		return fmt.Errorf("id is required")
	}
	if strings.TrimSpace(profile.Version) == "" {
		return fmt.Errorf("version is required")
	}
	if len(profile.Stack) == 0 {
		return fmt.Errorf("stack must contain at least one technology")
	}
	if len(profile.Commands) == 0 {
		return fmt.Errorf("commands must contain at least one verification command")
	}
	if strings.TrimSpace(profile.FontPolicy) == "" {
		return fmt.Errorf("font_policy is required")
	}
	if !strings.Contains(strings.ToLower(profile.FontPolicy), "no") {
		return fmt.Errorf("font_policy must explicitly prohibit new fonts")
	}
	return nil
}

// RenderMemory creates the project memory snapshot consumed by Agentsync.
// The baseline hash is provenance only; it does not change the policy text.
func RenderMemory(profile ProfileData, baselineHash string) string {
	var b strings.Builder
	b.WriteString("<!-- Generated by agentsync governance; edit the canonical source, not rendered adapters. -->\n")
	fmt.Fprintf(&b, "<!-- baseline-sha256: %s -->\n\n", baselineHash)
	b.WriteString(Baseline())
	b.WriteString("\n\n## Project Profile\n\n")
	fmt.Fprintf(&b, "- Profile ID: `%s`\n- Profile version: `%s`\n- Package manager: `%s`\n- Stack: %s\n", profile.ID, profile.Version, profile.PackageManager, strings.Join(profile.Stack, ", "))
	b.WriteString("- Existing local patterns and explicit task constraints take precedence over this snapshot.\n")
	b.WriteString("- Run the Profile verification commands relevant to the changed surface and report skipped or failed checks.\n")
	b.WriteString("\n### Project-specific rules\n\n")
	for _, rule := range profile.Rules {
		fmt.Fprintf(&b, "- %s\n", rule)
	}
	return b.String()
}

// LocalIgnoreEntries returns generated project paths that should remain local
// runtime state. Existing canonical policy adapters are never added implicitly.
func LocalIgnoreEntries(profile ProfileData) []string {
	entries := []string{".agent-governance/", ".agentsync/"}
	entries = append(entries, profile.LocalAdapters...)
	seen := make(map[string]struct{}, len(entries))
	result := make([]string, 0, len(entries))
	for _, entry := range entries {
		if _, ok := seen[entry]; ok {
			continue
		}
		seen[entry] = struct{}{}
		result = append(result, entry)
	}
	return result
}

// Capabilities returns the explicit capability report for an agent.
func Capabilities(agent string) (CapabilityReport, bool) {
	data, err := assets.ReadFile("assets/capabilities.json")
	if err != nil {
		panic(fmt.Sprintf("embedded capability registry missing: %v", err))
	}
	var registry struct {
		Agents map[string]CapabilityReport `json:"agents"`
	}
	if err := json.Unmarshal(data, &registry); err != nil {
		panic(fmt.Sprintf("parse embedded capability registry: %v", err))
	}
	report, ok := registry.Agents[agent]
	return report, ok
}
