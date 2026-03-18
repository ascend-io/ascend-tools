#!/usr/bin/env node
try {
  require('./index.js').runCli(["ascend-tools", ...process.argv.slice(2)])
} catch (e) {
  console.error(`Error: ${e.message}`)
  process.exitCode = 1
}
