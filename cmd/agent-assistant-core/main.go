// agent-assistant-core is the local JSON-line sidecar for the macOS client.
// It has no network listener and reads only the user's canonical files.
package main

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/spxrogers/agentsync/internal/desktopcore"
	"github.com/spxrogers/agentsync/internal/paths"
)

type request struct {
	Method  string          `json:"method"`
	Payload json.RawMessage `json:"payload,omitempty"`
}

func main() {
	home := paths.AgentsyncHome(paths.OSEnv{})
	userHome := paths.HomeDir(paths.OSEnv{})
	projectsRoot := os.Getenv("AGENT_ASSISTANT_PROJECTS_ROOT")
	if projectsRoot == "" {
		projectsRoot = filepath.Join(userHome, "git", "work")
	}
	options := desktopcore.Options{Home: home, ProjectsRoot: projectsRoot}
	ruleCore := desktopcore.NewRuleCore(options, desktopcore.NewCodexRuleAnalyzer(nil))
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
		case "rules_get":
			var payload desktopcore.RuleRequest
			if !decodePayload(encoder, req.Payload, &payload) {
				continue
			}
			result, err := ruleCore.GetRules(payload)
			if err != nil {
				writeError(encoder, err)
				continue
			}
			_ = encoder.Encode(result)
		case "rules_save":
			var payload desktopcore.SaveRuleRequest
			if !decodePayload(encoder, req.Payload, &payload) {
				continue
			}
			result, err := ruleCore.SaveRule(payload)
			if err != nil {
				writeError(encoder, err)
				continue
			}
			_ = encoder.Encode(result)
		case "rules_sync":
			var payload desktopcore.SyncRulesRequest
			if !decodePayload(encoder, req.Payload, &payload) {
				continue
			}
			result, err := ruleCore.SyncRules(payload)
			if err != nil {
				writeError(encoder, err)
				continue
			}
			_ = encoder.Encode(result)
		case "rules_import_native":
			var payload desktopcore.ImportNativeRuleRequest
			if !decodePayload(encoder, req.Payload, &payload) {
				continue
			}
			result, err := ruleCore.ImportNativeRule(payload)
			if err != nil {
				writeError(encoder, err)
				continue
			}
			_ = encoder.Encode(result)
		case "project_import":
			var payload struct {
				Path string `json:"path"`
			}
			if !decodePayload(encoder, req.Payload, &payload) {
				continue
			}
			result, err := ruleCore.ImportProject(payload.Path)
			if err != nil {
				writeError(encoder, err)
				continue
			}
			_ = encoder.Encode(result)
		case "project_analyze_rules":
			var payload struct {
				Path string `json:"path"`
			}
			if !decodePayload(encoder, req.Payload, &payload) {
				continue
			}
			result, err := ruleCore.AnalyzeProjectRules(context.Background(), payload.Path)
			if err != nil {
				writeError(encoder, err)
				continue
			}
			_ = encoder.Encode(result)
		default:
			writeError(encoder, fmt.Errorf("unknown method %q", req.Method))
		}
	}
}

func decodePayload(encoder *json.Encoder, payload json.RawMessage, target any) bool {
	if len(payload) == 0 {
		writeError(encoder, fmt.Errorf("request payload is required"))
		return false
	}
	if err := json.Unmarshal(payload, target); err != nil {
		writeError(encoder, fmt.Errorf("invalid request payload: %w", err))
		return false
	}
	return true
}

func writeError(encoder *json.Encoder, err error) {
	_ = encoder.Encode(map[string]string{"error": err.Error()})
}
