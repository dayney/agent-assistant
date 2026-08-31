// Package adapterregistry wires every production Agent adapter into one shared
// registry. Both the CLI and desktop Core use this package so their supported
// Agent set and verified destination paths cannot drift apart.
package adapterregistry

import (
	"fmt"

	"github.com/spxrogers/agentsync/internal/adapter"
	"github.com/spxrogers/agentsync/internal/adapter/claude"
	"github.com/spxrogers/agentsync/internal/adapter/cline"
	"github.com/spxrogers/agentsync/internal/adapter/codex"
	"github.com/spxrogers/agentsync/internal/adapter/continuedev"
	"github.com/spxrogers/agentsync/internal/adapter/cursor"
	"github.com/spxrogers/agentsync/internal/adapter/gemini"
	"github.com/spxrogers/agentsync/internal/adapter/generic"
	"github.com/spxrogers/agentsync/internal/adapter/opencode"
	"github.com/spxrogers/agentsync/internal/adapter/roo"
	"github.com/spxrogers/agentsync/internal/adapter/windsurf"
)

// New returns the complete production registry rooted at targetRoot. A
// registration collision is a programming error, so it panics rather than
// silently selecting one adapter.
func New(targetRoot string) *adapter.Registry {
	r := adapter.NewRegistry()
	mustRegister := func(a adapter.Adapter) {
		if err := r.Register(a); err != nil {
			panic(fmt.Errorf("agentsync: adapter registry wiring bug: %w", err))
		}
	}
	mustRegister(claude.New(claude.Options{TargetRoot: targetRoot}))
	mustRegister(opencode.New(opencode.Options{TargetRoot: targetRoot}))
	mustRegister(codex.New(codex.Options{TargetRoot: targetRoot}))
	mustRegister(cursor.New(cursor.Options{TargetRoot: targetRoot}))
	mustRegister(gemini.New(gemini.Options{TargetRoot: targetRoot}))
	mustRegister(continuedev.New(continuedev.Options{TargetRoot: targetRoot}))
	mustRegister(windsurf.New(windsurf.Options{TargetRoot: targetRoot}))
	mustRegister(roo.New(roo.Options{TargetRoot: targetRoot}))
	mustRegister(cline.New(cline.Options{TargetRoot: targetRoot}))
	for _, spec := range generic.Specs() {
		mustRegister(generic.New(spec, generic.Options{TargetRoot: targetRoot}))
	}
	return r
}
