package cli

import (
	"github.com/spxrogers/agentsync/internal/adapter"
	"github.com/spxrogers/agentsync/internal/adapterregistry"
	"github.com/spxrogers/agentsync/internal/paths"
)

// registryFactory returns an adapter.Registry wired with the production
// adapters. Tests reassign this var directly to inject a stub registry. Every
// valid agent (see validateAgent) now has a real adapter — there are no noop
// placeholders.
//
// A Register error only ever means a name collision (two registrations claiming
// the same Name()) — a wiring bug, never a user-recoverable runtime condition —
// so mustRegister panics on it rather than silently letting the first
// registration win and mis-resolving Lookup for that agent. The subset guard in
// registry_internal_test.go pins this at CI time; the panic is the runtime
// backstop. See #160.
var registryFactory = func() *adapter.Registry {
	return adapterregistry.New(paths.HomeDir(paths.OSEnv{}))
}
