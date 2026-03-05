## @itsbjoern/roost

This is the npm wrapper for the Roost CLI. It downloads a prebuilt `roost` binary from GitHub Releases and exposes the `roost` command. You can also use the **JavaScript API** from Node without shelling out.

### Install

```bash
# Global install — use `roost` from anywhere:
npm install -g @itsbjoern/roost

# Local install — use via npx:
npm install @itsbjoern/roost
```

Then (CLI):

```bash
roost init          # after global install
npx roost init      # after local install
```

### JavaScript API

Use the package as a library to get cert/key **contents** (or paths) for HTTPS config. Use `getDomainCerts` for build tools—returns literal PEM strings, no file reads:

```js
const { getDomainCerts } = require('@itsbjoern/roost');

const { cert, key } = await getDomainCerts('api.local', { generate: true });
// Pass cert and key directly to https.createServer or Vite server.https
```

See the [main README](https://github.com/itsbjoern/roost#javascript-api) for full API docs and examples.

