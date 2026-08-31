// agent-assistant-core is the local JSON-line sidecar for the macOS client.
// It has no network listener and reads only the user's canonical files.
package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/spxrogers/agentsync/internal/desktopcore"
	"github.com/spxrogers/agentsync/internal/paths"
)

type request struct {
	Method string `json:"method"`
}

func main() {
	home := paths.AgentsyncHome(paths.OSEnv{})
	userHome := paths.HomeDir(paths.OSEnv{})
	projectsRoot := os.Getenv("AGENT_ASSISTANT_PROJECTS_ROOT")
	if projectsRoot == "" {
		projectsRoot = filepath.Join(userHome, "git", "work")
	}
	options := desktopcore.Options{Home: home, ProjectsRoot: projectsRoot}
	encoder := json.NewEncoder(os.Stdout)
	scanner := bufio.NewScanner(os.Stdin)
	for scanner.Scan() {
		var req request
		if err := json.Unmarshal(scanner.Bytes(), &req); err != nil {
			writeError(encoder, fmt.Errorf("invalid request: %w", err))
			continue
		}
		switch strings.ToLower(req.Method) {
		case "snapshot", "get_workspace_snapshot":
			snapshot, err := desktopcore.ReadSnapshot(options)
			if err != nil {
				writeError(encoder, err)
				continue
			}
			_ = encoder.Encode(snapshot)
		case "preview", "preview_apply":
			snapshot, err := desktopcore.ReadSnapshot(options)
			if err != nil {
				writeError(encoder, err)
				continue
			}
			_ = encoder.Encode(desktopcore.PreviewApply(snapshot))
		default:
			writeError(encoder, fmt.Errorf("unknown method %q", req.Method))
		}
	}
}

func writeError(encoder *json.Encoder, err error) {
	_ = encoder.Encode(map[string]string{"error": err.Error()})
}
