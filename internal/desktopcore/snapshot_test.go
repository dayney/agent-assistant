package desktopcore

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/spxrogers/agentsync/internal/state"
	"github.com/spxrogers/agentsync/internal/testenv"
)

func TestReadSnapshotDiscoversProjectCanonicalTrees(t *testing.T) {
	testenv.RequireContainer(t)
	root := t.TempDir()
	global := filepath.Join(root, "global")
	projects := filepath.Join(root, "projects")
	if err := os.MkdirAll(global, 0o755); err != nil {
		t.Fatal(err)
	}
	projectRoot := filepath.Join(projects, "example")
	if err := os.MkdirAll(filepath.Join(projectRoot, ".agentsync"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(projectRoot, ".agentsync", "agentsync.toml"), []byte("[agents]\ncodex = { enabled = true }\n"), 0o644); err != nil {
		t.Fatal(err)
	}

	snapshot, err := ReadSnapshot(Options{Home: global, ProjectsRoot: projects, NativeRoot: root})
	if err != nil {
		t.Fatal(err)
	}
	if snapshot.SchemaVersion != 1 || snapshot.Mode != "real" || len(snapshot.Projects) != 1 || len(snapshot.Agents) != 1 {
		t.Fatalf("unexpected snapshot: %+v", snapshot)
	}
	if snapshot.Agents[0].Name != "Codex" {
		t.Fatalf("agent = %q, want Codex", snapshot.Agents[0].Name)
	}
}

func TestReadSnapshotIncludesManuallyImportedProjectWithoutCanonicalTree(t *testing.T) {
	testenv.RequireContainer(t)
	root := t.TempDir()
	global := filepath.Join(root, "global")
	projectRoot := filepath.Join(root, "outside-discovery", "plain-project")
	if err := os.MkdirAll(projectRoot, 0o755); err != nil {
		t.Fatal(err)
	}
	registryPath := filepath.Join(global, ".state", "agent-assistant", "projects.json")
	if err := state.SaveProjectRegistry(registryPath, &state.ProjectRegistry{
		Projects: []state.RegisteredProject{{Path: projectRoot, AddedAt: time.Now().UTC()}},
	}); err != nil {
		t.Fatal(err)
	}

	snapshot, err := ReadSnapshot(Options{Home: global, ProjectsRoot: filepath.Join(root, "empty"), NativeRoot: root})
	if err != nil {
		t.Fatal(err)
	}
	if len(snapshot.Projects) != 1 || snapshot.Projects[0].Path != projectRoot || snapshot.Projects[0].SyncState != "not-configured" {
		t.Fatalf("manual project was not represented faithfully: %+v", snapshot.Projects)
	}
}

func TestReadSnapshotRedactsNativeMCPURLAndKeepsSecretReferences(t *testing.T) {
	testenv.RequireContainer(t)
	root := t.TempDir()
	global := filepath.Join(root, "global")
	projects := filepath.Join(root, "projects")
	if err := os.MkdirAll(filepath.Join(global, "mcp"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(filepath.Join(root, ".cursor"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(global, "mcp", "remote.toml"), []byte("[server]\ntype = \"http\"\nurl = \"https://example.test/mcp?token=do-not-show\"\n[server.headers]\nAuthorization = \"${env:MCP_TOKEN}\"\n"), 0o644); err != nil {
		t.Fatal(err)
	}

	snapshot, err := ReadSnapshot(Options{Home: global, ProjectsRoot: projects, NativeRoot: root})
	if err != nil {
		t.Fatal(err)
	}
	if len(snapshot.Global.MCP) != 1 {
		t.Fatalf("global MCP count = %d, want 1", len(snapshot.Global.MCP))
	}
	item := snapshot.Global.MCP[0]
	if strings.Contains(item.Endpoint, "do-not-show") {
		t.Fatalf("endpoint leaked URL query: %q", item.Endpoint)
	}
	if len(item.SecretRefs) != 1 || item.SecretRefs[0] != "${env:MCP_TOKEN}" {
		t.Fatalf("secret refs = %#v, want the reference only", item.SecretRefs)
	}
}
