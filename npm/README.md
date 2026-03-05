## @itsbjoern/roost

CLI and JavaScript API for Roost (local HTTPS reverse proxy with automatic cert management). The package ships with platform-specific binaries (like [Biome](https://github.com/biomejs/biome)); after install you can run `roost` directly.

### Install

```bash
npm install -g @itsbjoern/roost
```

### Usage

```bash
roost init          # after global install
npx roost init      # without installing (npm)
```

If `roost` is not found after a global install, add your package manager's global bin to `PATH` (e.g. `export PATH="$PATH:$(npm config get prefix)/bin"` for npm, or `export PATH="$PATH:$HOME/.bun/bin"` for Bun).

### JavaScript API

Use the package as a library to get cert/key **contents** (or paths) for HTTPS config. Use `getDomainCerts` for build tools—returns literal PEM strings, no file reads:

```js
const { getDomainCerts } = require('@itsbjoern/roost');

const { cert, key } = await getDomainCerts('api.local', { generate: true });
// Pass cert and key directly to https.createServer or Vite server.https
```

See the [main README](https://github.com/itsbjoern/roost#javascript-api) for full API docs and examples.
