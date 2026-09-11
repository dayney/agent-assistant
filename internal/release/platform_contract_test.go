package release

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"testing"

	"sigs.k8s.io/yaml"
)

type platformPolicy struct {
	SchemaVersion int `json:"schemaVersion"`
	Toolchains    struct {
		Go         string `json:"go"`
		Node       string `json:"node"`
		Rust       string `json:"rust"`
		GoReleaser string `json:"goreleaser"`
	} `json:"toolchains"`
	Targets []platformTarget `json:"targets"`
}

type platformTarget struct {
	ID             string `json:"id"`
	Product        string `json:"product"`
	OS             string `json:"os"`
	Arch           string `json:"arch"`
	MinimumVersion string `json:"minimumVersion"`
	MaximumVersion string `json:"maximumVersion,omitempty"`
	Tier           string `json:"tier"`
	Runner         string `json:"runner,omitempty"`
	Cadence        string `json:"cadence"`
}

type workflow struct {
	Jobs map[string]struct {
		Strategy struct {
			Matrix struct {
				Include []workflowTarget `json:"include"`
			} `json:"matrix"`
		} `json:"strategy"`
	} `json:"jobs"`
}

type workflowTarget struct {
	Product string `json:"product"`
	OS      string `json:"os"`
	Arch    string `json:"arch"`
	Tier    string `json:"tier"`
	Runner  string `json:"runner"`
}

func TestPlatformPolicy(t *testing.T) {
	policy := readPlatformPolicy(t)
	if policy.SchemaVersion != 1 {
		t.Fatalf("schemaVersion = %d, want 1", policy.SchemaVersion)
	}

	want := map[string]platformTarget{
		"cli-macos-arm64":          {Product: "cli", OS: "darwin", Arch: "arm64", MinimumVersion: "14", Tier: "supported", Runner: "macos-14", Cadence: "required"},
		"cli-macos-amd64":          {Product: "cli", OS: "darwin", Arch: "amd64", MinimumVersion: "14", Tier: "supported", Runner: "macos-15-intel", Cadence: "required"},
		"cli-macos-arm64-compat":   {Product: "cli", OS: "darwin", Arch: "arm64", MinimumVersion: "12", MaximumVersion: "13", Tier: "compatible", Cadence: "none"},
		"cli-macos-amd64-compat":   {Product: "cli", OS: "darwin", Arch: "amd64", MinimumVersion: "12", MaximumVersion: "13", Tier: "compatible", Cadence: "none"},
		"cli-windows-amd64":        {Product: "cli", OS: "windows", Arch: "amd64", MinimumVersion: "11 25H2", Tier: "supported", Runner: "windows-2022", Cadence: "required"},
		"cli-windows-amd64-compat": {Product: "cli", OS: "windows", Arch: "amd64", MinimumVersion: "10", MaximumVersion: "11 24H2", Tier: "compatible", Cadence: "none"},
		"cli-windows-arm64":        {Product: "cli", OS: "windows", Arch: "arm64", MinimumVersion: "11 25H2", Tier: "preview", Runner: "windows-11-arm", Cadence: "scheduled"},
		"desktop-macos-arm64":      {Product: "desktop", OS: "darwin", Arch: "arm64", MinimumVersion: "14", Tier: "supported", Runner: "macos-14", Cadence: "required"},
		"desktop-macos-amd64":      {Product: "desktop", OS: "darwin", Arch: "amd64", MinimumVersion: "14", Tier: "supported", Runner: "macos-15-intel", Cadence: "required"},
		"desktop-windows-amd64":    {Product: "desktop", OS: "windows", Arch: "amd64", MinimumVersion: "11 25H2", Tier: "supported", Runner: "windows-2022", Cadence: "required"},
		"desktop-windows-arm64":    {Product: "desktop", OS: "windows", Arch: "arm64", MinimumVersion: "11 25H2", Tier: "preview", Runner: "windows-11-arm", Cadence: "scheduled"},
	}

	if len(policy.Targets) != len(want) {
		t.Fatalf("target count = %d, want %d", len(policy.Targets), len(want))
	}
	seen := make(map[string]bool, len(policy.Targets))
	for _, target := range policy.Targets {
		if seen[target.ID] {
			t.Errorf("duplicate target id %q", target.ID)
			continue
		}
		seen[target.ID] = true
		if target.Tier != "supported" && target.Tier != "compatible" && target.Tier != "preview" {
			t.Errorf("target %q has invalid tier %q", target.ID, target.Tier)
		}
		if target.Tier == "compatible" && (target.Runner != "" || target.Cadence != "none") {
			t.Errorf("compatible target %q must not claim continuous CI coverage", target.ID)
		}
		if target.Tier != "compatible" && (target.Runner == "" || target.Cadence == "none") {
			t.Errorf("%s target %q must name a validation runner and cadence", target.Tier, target.ID)
		}

		expected, ok := want[target.ID]
		if !ok {
			t.Errorf("unexpected target %q", target.ID)
			continue
		}
		id := target.ID
		target.ID = ""
		if target != expected {
			t.Errorf("target %q = %+v, want %+v", id, target, expected)
		}
	}
	for id := range want {
		if !seen[id] {
			t.Errorf("missing target %q", id)
		}
	}
	assertCompatibleRangesDoNotOverlapSupported(t, policy.Targets)
}

func assertCompatibleRangesDoNotOverlapSupported(t *testing.T, targets []platformTarget) {
	t.Helper()
	supportedMinimum := make(map[string]int)
	for _, target := range targets {
		if target.Tier != "supported" {
			continue
		}
		key := target.Product + "/" + target.OS + "/" + target.Arch
		supportedMinimum[key] = platformVersionRank(t, target.OS, target.MinimumVersion)
	}
	for _, target := range targets {
		if target.Tier != "compatible" {
			continue
		}
		key := target.Product + "/" + target.OS + "/" + target.Arch
		supported, ok := supportedMinimum[key]
		if !ok {
			continue
		}
		if target.MaximumVersion == "" {
			t.Errorf("compatible target %q must end before its supported range", target.ID)
			continue
		}
		minimum := platformVersionRank(t, target.OS, target.MinimumVersion)
		maximum := platformVersionRank(t, target.OS, target.MaximumVersion)
		if minimum > maximum {
			t.Errorf("compatible target %q has an inverted version range", target.ID)
		}
		if maximum >= supported {
			t.Errorf("compatible target %q overlaps its supported range", target.ID)
		}
	}
}

func platformVersionRank(t *testing.T, osName, version string) int {
	t.Helper()
	if osName == "darwin" {
		value, err := strconv.Atoi(version)
		if err != nil {
			t.Fatalf("invalid macOS version %q: %v", version, err)
		}
		return value
	}
	if osName == "windows" {
		match := regexp.MustCompile(`^(10|11)(?: ([0-9]{2})H([12]))?$`).FindStringSubmatch(version)
		if match == nil {
			t.Fatalf("invalid Windows version %q", version)
		}
		major, _ := strconv.Atoi(match[1])
		if match[2] == "" {
			return major * 1000
		}
		year, _ := strconv.Atoi(match[2])
		half, _ := strconv.Atoi(match[3])
		return major*1000 + year*2 + half
	}
	t.Fatalf("unsupported policy OS %q", osName)
	return 0
}

func TestPlatformToolchainPins(t *testing.T) {
	root := findRepoRoot(t)
	policy := readPlatformPolicy(t)

	assertFileEquals(t, filepath.Join(root, ".node-version"), policy.Toolchains.Node)
	assertRegexpCapture(t, filepath.Join(root, "go.mod"), `(?m)^go ([0-9]+\.[0-9]+\.[0-9]+)$`, policy.Toolchains.Go)
	assertRegexpCapture(t, filepath.Join(root, "test", "container", "Containerfile"), `(?m)^ARG GO_VERSION=([0-9]+\.[0-9]+\.[0-9]+)$`, policy.Toolchains.Go)
	assertRegexpCapture(t, filepath.Join(root, "rust-toolchain.toml"), `(?m)^channel = "([0-9]+\.[0-9]+\.[0-9]+)"$`, policy.Toolchains.Rust)
	justfilePath := filepath.Join(root, "justfile")
	assertRegexpCapture(t, justfilePath, `goreleaser/goreleaser/v2@v([0-9]+\.[0-9]+\.[0-9]+) release`, policy.Toolchains.GoReleaser)
	if !strings.Contains(string(readFile(t, justfilePath)), "--skip=publish,chocolatey,sign") {
		t.Error("justfile GoReleaser snapshot must skip the same unavailable publish tools as CI")
	}
	reproducibilityPath := filepath.Join(root, "scripts", "reproducibility-diff.sh")
	assertRegexpCapture(t, reproducibilityPath, `GORELEASER="github.com/goreleaser/goreleaser/v2@v([0-9]+\.[0-9]+\.[0-9]+)"`, policy.Toolchains.GoReleaser)
	if !strings.Contains(string(readFile(t, reproducibilityPath)), "--skip=publish,chocolatey,sign") {
		t.Error("reproducibility snapshot must skip the same unavailable publish tools as CI")
	}
	assertRegexpCapture(t, filepath.Join(root, "CONTRIBUTING.md"), `currently ([0-9]+\.[0-9]+\.[0-9]+)\)`, policy.Toolchains.Go)

	var desktopPackage struct {
		Engines struct {
			Node string `json:"node"`
		} `json:"engines"`
	}
	packagePath := filepath.Join(root, "desktop", "package.json")
	if err := json.Unmarshal(readFile(t, packagePath), &desktopPackage); err != nil {
		t.Fatalf("parsing desktop/package.json: %v", err)
	}
	if desktopPackage.Engines.Node != ">="+policy.Toolchains.Node {
		t.Errorf("desktop Node engine = %q, want >=%s", desktopPackage.Engines.Node, policy.Toolchains.Node)
	}

	for _, name := range []string{"ci.yml", "release.yml"} {
		path := filepath.Join(root, ".github", "workflows", name)
		raw := readFile(t, path)
		matches := regexp.MustCompile(`(?m)^\s+version: "([0-9]+\.[0-9]+\.[0-9]+)"$`).FindAllStringSubmatch(string(raw), -1)
		if len(matches) == 0 {
			t.Errorf("%s has no pinned GoReleaser version", name)
		}
		for _, match := range matches {
			if match[1] != policy.Toolchains.GoReleaser {
				t.Errorf("%s GoReleaser pin = %s, want %s", name, match[1], policy.Toolchains.GoReleaser)
			}
		}
	}
}

func TestPlatformReleaseTargets(t *testing.T) {
	root := findRepoRoot(t)
	policy := readPlatformPolicy(t)
	raw := readFile(t, filepath.Join(root, ".goreleaser.yaml"))
	var config struct {
		Builds []struct {
			Goos   []string `json:"goos"`
			Goarch []string `json:"goarch"`
		} `json:"builds"`
	}
	if err := yaml.Unmarshal(raw, &config); err != nil {
		t.Fatalf("parsing .goreleaser.yaml: %v", err)
	}
	if len(config.Builds) != 1 {
		t.Fatalf("GoReleaser build count = %d, want 1", len(config.Builds))
	}

	got := crossProduct(config.Builds[0].Goos, config.Builds[0].Goarch)
	want := map[string]bool{"linux/amd64": true, "linux/arm64": true}
	for _, target := range policy.Targets {
		if target.Product == "cli" {
			want[target.OS+"/"+target.Arch] = true
		}
	}
	assertStringSet(t, "GoReleaser targets", got, want)
}

func TestPlatformRequiredCI(t *testing.T) {
	policy := readPlatformPolicy(t)
	workflow := readWorkflow(t, ".github/workflows/ci.yml")

	for _, job := range []string{"test-fast", "desktop-native"} {
		matrix, ok := workflow.Jobs[job]
		if !ok {
			t.Errorf("required CI job %q is missing", job)
			continue
		}
		got := workflowSet(matrix.Strategy.Matrix.Include)
		want := make(map[string]bool)
		product := "cli"
		if job == "desktop-native" {
			product = "desktop"
		}
		for _, target := range policy.Targets {
			if target.Product == product && target.Tier == "supported" {
				want[workflowKey(workflowTarget{
					Product: target.Product,
					OS:      target.OS,
					Arch:    target.Arch,
					Tier:    target.Tier,
					Runner:  target.Runner,
				})] = true
			}
		}
		if job == "test-fast" {
			want[workflowKey(workflowTarget{Product: "cli", OS: "linux", Arch: "amd64", Tier: "supported", Runner: "ubuntu-24.04"})] = true
		}
		assertStringSet(t, job+" matrix", got, want)
		for _, row := range matrix.Strategy.Matrix.Include {
			if strings.Contains(row.Runner, "latest") {
				t.Errorf("required job %q uses floating runner %q", job, row.Runner)
			}
		}
	}
}

func TestPlatformCanaryCI(t *testing.T) {
	policy := readPlatformPolicy(t)
	workflow := readWorkflow(t, ".github/workflows/platform-canary.yml")
	job, ok := workflow.Jobs["canary"]
	if !ok {
		t.Fatal("platform canary job is missing")
	}
	got := workflowSet(job.Strategy.Matrix.Include)
	want := make(map[string]bool)
	for _, product := range []string{"cli", "desktop"} {
		want[workflowKey(workflowTarget{Product: product, OS: "darwin", Arch: "arm64", Tier: "canary", Runner: "macos-latest"})] = true
		want[workflowKey(workflowTarget{Product: product, OS: "windows", Arch: "amd64", Tier: "canary", Runner: "windows-latest"})] = true
	}
	for _, target := range policy.Targets {
		if target.Tier == "preview" {
			want[workflowKey(workflowTarget{Product: target.Product, OS: target.OS, Arch: target.Arch, Tier: target.Tier, Runner: target.Runner})] = true
		}
	}
	assertStringSet(t, "canary matrix", got, want)
}

func TestPlatformDesktopBundleConfigs(t *testing.T) {
	root := findRepoRoot(t)
	type tauriConfig struct {
		Bundle struct {
			Targets []string `json:"targets"`
			Icon    []string `json:"icon"`
			MacOS   struct {
				MinimumSystemVersion string `json:"minimumSystemVersion"`
			} `json:"macOS"`
			Windows struct {
				WebviewInstallMode struct {
					Type   string `json:"type"`
					Silent bool   `json:"silent"`
				} `json:"webviewInstallMode"`
			} `json:"windows"`
		} `json:"bundle"`
	}
	readConfig := func(name string) tauriConfig {
		t.Helper()
		var config tauriConfig
		path := filepath.Join(root, "desktop", "src-tauri", name)
		if err := json.Unmarshal(readFile(t, path), &config); err != nil {
			t.Fatalf("parsing %s: %v", name, err)
		}
		return config
	}

	macOS := readConfig("tauri.macos.conf.json")
	assertStringSet(t, "macOS bundle targets", sliceSet(macOS.Bundle.Targets), map[string]bool{"app": true, "dmg": true})
	if macOS.Bundle.MacOS.MinimumSystemVersion != "14.0" {
		t.Errorf("macOS Desktop minimum = %q, want 14.0", macOS.Bundle.MacOS.MinimumSystemVersion)
	}

	windows := readConfig("tauri.windows.conf.json")
	assertStringSet(t, "Windows bundle targets", sliceSet(windows.Bundle.Targets), map[string]bool{"nsis": true})
	if windows.Bundle.Windows.WebviewInstallMode.Type != "embedBootstrapper" || !windows.Bundle.Windows.WebviewInstallMode.Silent {
		t.Errorf("Windows WebView2 mode = %+v, want silent embedBootstrapper", windows.Bundle.Windows.WebviewInstallMode)
	}

	base := readConfig("tauri.conf.json")
	wantIcons := map[string]bool{
		"icons/32x32.png":      true,
		"icons/128x128.png":    true,
		"icons/128x128@2x.png": true,
		"icons/icon.icns":      true,
		"icons/icon.ico":       true,
	}
	assertStringSet(t, "Desktop bundle icons", sliceSet(base.Bundle.Icon), wantIcons)
	for icon := range wantIcons {
		if _, err := os.Stat(filepath.Join(root, "desktop", "src-tauri", filepath.FromSlash(icon))); err != nil {
			t.Errorf("declared Desktop icon %q is unavailable: %v", icon, err)
		}
	}
}

func TestPlatformDesktopReleaseIsFailClosed(t *testing.T) {
	root := findRepoRoot(t)
	release := string(readFile(t, filepath.Join(root, ".github", "workflows", "release.yml")))
	for _, fragment := range []string{
		"vars.DESKTOP_RELEASE_ENABLED == 'true'",
		"uses: ./.github/workflows/desktop-release.yml",
		"secrets: inherit",
		"Desktop distribution disabled",
	} {
		if !strings.Contains(release, fragment) {
			t.Errorf("release.yml is missing Desktop release gate %q", fragment)
		}
	}

	path := filepath.Join(root, ".github", "workflows", "desktop-release.yml")
	desktopRelease := string(readFile(t, path))
	for _, fragment := range []string{
		"preflight:",
		"actions/checkout@v7",
		"scripts/release-tag.sh --validate-only \"$TAG\"",
		"scripts/release-tag.sh --package-version \"$TAG\"",
		"APPLE_CERTIFICATE",
		"APPLE_CERTIFICATE_PASSWORD",
		"APPLE_SIGNING_IDENTITY",
		"APPLE_ID",
		"APPLE_PASSWORD",
		"APPLE_TEAM_ID",
		"WINDOWS_CERTIFICATE",
		"WINDOWS_CERTIFICATE_PASSWORD",
		"WINDOWS_TIMESTAMP_URL",
		"codesign --verify --deep --strict",
		"codesign --verify --verbose=2 \"$dmg\"",
		"xcrun notarytool submit \"$dmg\"",
		"xcrun stapler staple \"$dmg\"",
		"xcrun stapler validate",
		"signtool.exe",
		"gh release upload",
	} {
		if !strings.Contains(desktopRelease, fragment) {
			t.Errorf("desktop-release.yml is missing fail-closed release behavior %q", fragment)
		}
	}
	for _, forbidden := range []string{"continue-on-error: true", "signingIdentity\": \"-\"", "[[ ! \"$TAG\" =~"} {
		if strings.Contains(desktopRelease, forbidden) {
			t.Errorf("desktop-release.yml contains unsafe signing fallback %q", forbidden)
		}
	}
}

func TestPlatformWorkflowsUseCurrentCheckoutMajor(t *testing.T) {
	root := findRepoRoot(t)
	for _, name := range []string{"ci.yml", "desktop-release.yml", "docs-publish.yml", "platform-canary.yml", "release.yml"} {
		raw := string(readFile(t, filepath.Join(root, ".github", "workflows", name)))
		matches := regexp.MustCompile(`actions/checkout@v[0-9]+`).FindAllString(raw, -1)
		if len(matches) == 0 {
			t.Errorf("%s has no checkout action", name)
		}
		for _, match := range matches {
			if match != "actions/checkout@v7" {
				t.Errorf("%s uses %s, want actions/checkout@v7", name, match)
			}
		}
	}
}

func readPlatformPolicy(t *testing.T) platformPolicy {
	t.Helper()
	raw := readFile(t, filepath.Join(findRepoRoot(t), "platform-support.json"))
	var policy platformPolicy
	if err := json.Unmarshal(raw, &policy); err != nil {
		t.Fatalf("parsing platform-support.json: %v", err)
	}
	return policy
}

func readWorkflow(t *testing.T, relativePath string) workflow {
	t.Helper()
	raw := readFile(t, filepath.Join(findRepoRoot(t), relativePath))
	var config workflow
	if err := yaml.Unmarshal(raw, &config); err != nil {
		t.Fatalf("parsing %s: %v", relativePath, err)
	}
	return config
}

func readFile(t *testing.T, path string) []byte {
	t.Helper()
	raw, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("reading %s: %v", path, err)
	}
	return raw
}

func assertFileEquals(t *testing.T, path, want string) {
	t.Helper()
	got := strings.TrimSpace(string(readFile(t, path)))
	if got != want {
		t.Errorf("%s = %q, want %q", filepath.Base(path), got, want)
	}
}

func assertRegexpCapture(t *testing.T, path, pattern, want string) {
	t.Helper()
	match := regexp.MustCompile(pattern).FindSubmatch(readFile(t, path))
	if len(match) != 2 {
		t.Fatalf("%s does not match %s", filepath.Base(path), pattern)
	}
	if string(match[1]) != want {
		t.Errorf("%s pin = %q, want %q", filepath.Base(path), match[1], want)
	}
}

func crossProduct(oses, arches []string) map[string]bool {
	result := make(map[string]bool, len(oses)*len(arches))
	for _, osName := range oses {
		for _, arch := range arches {
			result[osName+"/"+arch] = true
		}
	}
	return result
}

func sliceSet(values []string) map[string]bool {
	result := make(map[string]bool, len(values))
	for _, value := range values {
		result[value] = true
	}
	return result
}

func workflowSet(rows []workflowTarget) map[string]bool {
	result := make(map[string]bool, len(rows))
	for _, row := range rows {
		result[workflowKey(row)] = true
	}
	return result
}

func workflowKey(target workflowTarget) string {
	return fmt.Sprintf("%s/%s/%s/%s/%s", target.Product, target.OS, target.Arch, target.Tier, target.Runner)
}

func assertStringSet(t *testing.T, label string, got, want map[string]bool) {
	t.Helper()
	if strings.Join(sortedKeys(got), "\n") != strings.Join(sortedKeys(want), "\n") {
		t.Errorf("%s mismatch\ngot:\n  %s\nwant:\n  %s", label, strings.Join(sortedKeys(got), "\n  "), strings.Join(sortedKeys(want), "\n  "))
	}
}

func sortedKeys(values map[string]bool) []string {
	keys := make([]string, 0, len(values))
	for value := range values {
		keys = append(keys, value)
	}
	sort.Strings(keys)
	return keys
}
