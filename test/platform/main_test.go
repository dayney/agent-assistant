// Package platform_test contains the native macOS and Windows CLI smoke test.
package platform_test

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"testing"

	"github.com/spxrogers/agentsync/internal/testenv"
)

const platformTestEnv = "AGENTSYNC_PLATFORM_TEST"

func TestMain(m *testing.M) {
	if !testenv.InContainer() && os.Getenv(platformTestEnv) != "1" {
		fmt.Fprintf(os.Stderr, "agentsync: native platform smoke requires %s=1 or the hermetic test container\n", platformTestEnv)
		os.Exit(1)
	}
	os.Exit(m.Run())
}

func TestNativeCLILifecycle(t *testing.T) {
	repositoryRoot := findRepositoryRoot(t)
	testRoot := t.TempDir()
	binaryName := "agentsync"
	if runtime.GOOS == "windows" {
		binaryName += ".exe"
	}
	binary := filepath.Join(testRoot, "bin", binaryName)
	if err := os.MkdirAll(filepath.Dir(binary), 0o755); err != nil {
		t.Fatalf("creating binary directory: %v", err)
	}

	build := exec.Command("go", "build", "-trimpath", "-o", binary, "./cmd/agentsync")
	build.Dir = repositoryRoot
	if output, err := build.CombinedOutput(); err != nil {
		t.Fatalf("building agentsync: %v\n%s", err, output)
	}

	sourceRoot := filepath.Join(testRoot, "source")
	targetRoot := filepath.Join(testRoot, "target")
	userHome := filepath.Join(testRoot, "home")
	workingDirectory := filepath.Join(testRoot, "work")
	for _, directory := range []string{targetRoot, userHome, workingDirectory} {
		if err := os.MkdirAll(directory, 0o755); err != nil {
			t.Fatalf("creating test directory %s: %v", directory, err)
		}
	}

	overrides := map[string]string{
		"AGENTSYNC_HOME":  sourceRoot,
		"USERPROFILE":     userHome,
		"XDG_CONFIG_HOME": filepath.Join(userHome, ".config"),
		"APPDATA":         filepath.Join(userHome, "AppData", "Roaming"),
		"LOCALAPPDATA":    filepath.Join(userHome, "AppData", "Local"),
		"NO_COLOR":        "1",
	}
	targetHome := userHome
	if runtime.GOOS != "windows" {
		overrides["AGENTSYNC_TARGET_ROOT"] = targetRoot
		overrides["HOME"] = userHome
		targetHome = targetRoot
	} else {
		// Windows GUI launches rely on USERPROFILE. Clear inherited test overrides
		// so this smoke cannot accidentally bypass that production path.
		overrides["HOME"] = ""
		overrides["AGENTSYNC_TARGET_ROOT"] = ""
	}
	runner := cliRunner{
		binary: binary,
		dir:    workingDirectory,
		env:    isolatedEnvironment(overrides),
		t:      t,
	}

	runner.run("init", "--no-input", "--color", "never")
	runner.run("agent", "add", "claude", "--no-input", "--color", "never")
	memoryPath := filepath.Join(sourceRoot, "memory", "AGENTS.md")
	if err := os.WriteFile(memoryPath, []byte("# Native platform smoke\n"), 0o644); err != nil {
		t.Fatalf("writing canonical memory fixture: %v", err)
	}
	runner.run("apply", "--no-input", "--no-git-backup", "--color", "never")
	status := runner.run("status", "--json", "--exit-code", "--no-input", "--color", "never")

	var report map[string]any
	if err := json.Unmarshal([]byte(status), &report); err != nil {
		t.Fatalf("decoding status JSON: %v\n%s", err, status)
	}
	renderedMemory := filepath.Join(targetHome, ".claude", "CLAUDE.md")
	content, err := os.ReadFile(renderedMemory)
	if err != nil {
		t.Fatalf("reading rendered Claude memory: %v", err)
	}
	if !strings.Contains(string(content), "Native platform smoke") {
		t.Fatalf("rendered Claude memory is missing the canonical fixture:\n%s", content)
	}
}

type cliRunner struct {
	binary string
	dir    string
	env    []string
	t      *testing.T
}

func (runner cliRunner) run(args ...string) string {
	runner.t.Helper()
	command := exec.Command(runner.binary, args...)
	command.Dir = runner.dir
	command.Env = runner.env
	output, err := command.CombinedOutput()
	if err != nil {
		runner.t.Fatalf("agentsync %s failed: %v\n%s", strings.Join(args, " "), err, output)
	}
	return strings.TrimSpace(string(output))
}

func isolatedEnvironment(overrides map[string]string) []string {
	removed := make(map[string]bool, len(overrides))
	for key := range overrides {
		removed[strings.ToUpper(key)] = true
	}
	environment := make([]string, 0, len(os.Environ())+len(overrides))
	for _, entry := range os.Environ() {
		key, _, ok := strings.Cut(entry, "=")
		if !ok || removed[strings.ToUpper(key)] {
			continue
		}
		environment = append(environment, entry)
	}
	for key, value := range overrides {
		environment = append(environment, key+"="+value)
	}
	return environment
}

func findRepositoryRoot(t *testing.T) string {
	t.Helper()
	directory, err := filepath.Abs(".")
	if err != nil {
		t.Fatalf("resolving test directory: %v", err)
	}
	for {
		if _, err := os.Stat(filepath.Join(directory, "go.mod")); err == nil {
			return directory
		}
		parent := filepath.Dir(directory)
		if parent == directory {
			t.Fatal("could not find repository root")
		}
		directory = parent
	}
}
