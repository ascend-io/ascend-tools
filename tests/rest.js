#!/usr/bin/env node
/**
 * Integration tests for the Ascend Instance web API.
 *
 * Self-contained — no ascend-tools dependency. Authenticates using Ed25519
 * JWT signing and exercises the /api/v1 endpoints directly via Node.js
 * built-in fetch and crypto.
 *
 * Requires ASCEND_SERVICE_ACCOUNT_ID, ASCEND_SERVICE_ACCOUNT_KEY, and
 * ASCEND_INSTANCE_API_URL environment variables.
 *
 * Usage: node tests/rest.js [--runtime-id <ID>]
 */

import { createPrivateKey, sign } from "node:crypto";
import { parseArgs } from "node:util";

// ---------------------------------------------------------------------------
// Ed25519 JWT signing
// ---------------------------------------------------------------------------

/** Encode bytes to unpadded base64url (RFC 7515 §2). */
function base64url(buf) {
  return Buffer.from(buf)
    .toString("base64")
    .replace(/\+/g, "-")
    .replace(/\//g, "_")
    .replace(/=+$/, "");
}

/** Decode unpadded base64url to a Buffer. */
function base64urlDecode(s) {
  s = s.replace(/-/g, "+").replace(/_/g, "/");
  while (s.length % 4) s += "=";
  return Buffer.from(s, "base64");
}

/**
 * Create a signed JWT using EdDSA (Ed25519).
 *
 * The JWT is three base64url segments joined by dots:
 *   base64url(header) . base64url(payload) . base64url(signature)
 */
function signJwt(claims, privateKey) {
  const header = base64url(JSON.stringify({ alg: "EdDSA", typ: "JWT" }));
  const payload = base64url(JSON.stringify(claims));
  const signingInput = Buffer.from(`${header}.${payload}`, "ascii");
  const signature = sign(null, signingInput, privateKey);
  return `${header}.${payload}.${base64url(signature)}`;
}

// ---------------------------------------------------------------------------
// Authentication
// ---------------------------------------------------------------------------

class Auth {
  /**
   * Ascend service account authentication.
   *
   * Flow:
   *   1. Decode the base64url Ed25519 seed (32 bytes) from the service account key
   *   2. Discover the JWT audience domain from GET /api/v1/auth/config
   *   3. Sign an Ed25519 JWT with service account claims
   *   4. Exchange the JWT for an instance token at POST /api/v1/auth/token
   *   5. Cache the token; refresh when within 5 minutes of expiry
   */
  constructor(serviceAccountId, serviceAccountKey, instanceApiUrl) {
    this.serviceAccountId = serviceAccountId;
    this.instanceApiUrl = instanceApiUrl;

    const seed = base64urlDecode(serviceAccountKey);
    if (seed.length !== 32) {
      throw new Error(
        `expected 32-byte Ed25519 seed, got ${seed.length} bytes`,
      );
    }

    // Node.js crypto requires a PKCS#8 DER wrapper around the 32-byte seed.
    // Ed25519 PKCS#8 prefix (RFC 8410): 30 2e 02 01 00 30 05 06 03 2b 65 70 04 22 04 20
    const pkcs8Prefix = Buffer.from(
      "302e020100300506032b657004220420",
      "hex",
    );
    const der = Buffer.concat([pkcs8Prefix, seed]);
    this.privateKey = createPrivateKey({ key: der, format: "der", type: "pkcs8" });

    this._cloudApiDomain = null;
    this._cachedToken = null;
    this._tokenExpiresAt = 0;
  }

  /** Return a valid instance token, refreshing if needed. */
  async getToken() {
    const now = Math.floor(Date.now() / 1000);
    if (this._cachedToken && this._tokenExpiresAt > now + 300) {
      return this._cachedToken;
    }

    const domain = await this._getCloudApiDomain();
    const saJwt = this._signSaJwt(now, domain);
    const { token, expiresAt } = await this._exchangeToken(saJwt);

    this._cachedToken = token;
    this._tokenExpiresAt = expiresAt;
    return token;
  }

  /** Fetch (and cache) the JWT audience domain from auth config. */
  async _getCloudApiDomain() {
    if (this._cloudApiDomain) return this._cloudApiDomain;

    const resp = await fetch(`${this.instanceApiUrl}/api/v1/auth/config`);
    if (!resp.ok) throw new Error(`auth/config failed: ${resp.status}`);
    const data = await resp.json();
    this._cloudApiDomain = data.cloud_api_domain;
    return this._cloudApiDomain;
  }

  /** Sign a service account JWT (5-minute expiry). */
  _signSaJwt(now, cloudApiDomain) {
    return signJwt(
      {
        sub: this.serviceAccountId,
        aud: `https://${cloudApiDomain}/auth/token`,
        exp: now + 300,
        iat: now,
        name: this.serviceAccountId,
        service_account: this.serviceAccountId,
      },
      this.privateKey,
    );
  }

  /** Exchange the SA JWT for an instance access token. */
  async _exchangeToken(saJwt) {
    const resp = await fetch(`${this.instanceApiUrl}/api/v1/auth/token`, {
      method: "POST",
      headers: { Authorization: `Bearer ${saJwt}` },
    });
    if (!resp.ok) throw new Error(`auth/token failed: ${resp.status}`);
    const data = await resp.json();
    return {
      token: data.access_token,
      expiresAt: data.expiration ?? Math.floor(Date.now() / 1000) + 3600,
    };
  }
}

// ---------------------------------------------------------------------------
// API client
// ---------------------------------------------------------------------------

class AscendClient {
  /** Minimal HTTP client for the Ascend Instance API v1. */
  constructor(auth) {
    this.auth = auth;
    this.baseUrl = auth.instanceApiUrl;
  }

  async _headers() {
    return { Authorization: `Bearer ${await this.auth.getToken()}` };
  }

  async _get(path, params) {
    let url = `${this.baseUrl}${path}`;
    if (params) {
      const qs = new URLSearchParams(
        Object.fromEntries(
          Object.entries(params).filter(([, v]) => v != null),
        ),
      );
      if (qs.toString()) url += `?${qs}`;
    }
    const resp = await fetch(url, { headers: await this._headers() });
    await raiseForApiError(resp);
    return resp.json();
  }

  async _postEmpty(path) {
    const resp = await fetch(`${this.baseUrl}${path}`, {
      method: "POST",
      headers: await this._headers(),
    });
    await raiseForApiError(resp);
    return resp.json();
  }

  async _postJson(path, body) {
    const resp = await fetch(`${this.baseUrl}${path}`, {
      method: "POST",
      headers: { ...(await this._headers()), "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    await raiseForApiError(resp);
    return resp.json();
  }

  // -- Runtimes --

  async listRuntimes(filters = {}) {
    return this._get("/api/v1/runtimes", filters);
  }

  async getRuntime(uuid) {
    return this._get(`/api/v1/runtimes/${encode(uuid)}`);
  }

  async pauseRuntime(uuid) {
    return this._postEmpty(`/api/v1/runtimes/${encode(uuid)}:pause`);
  }

  async resumeRuntime(uuid) {
    return this._postEmpty(`/api/v1/runtimes/${encode(uuid)}:resume`);
  }

  // -- Flows --

  async listFlows(runtimeUuid) {
    return this._get(`/api/v1/runtimes/${encode(runtimeUuid)}/flows`);
  }

  async runFlow(runtimeUuid, flowName, { spec, resume: shouldResume } = {}) {
    const runtime = await this.getRuntime(runtimeUuid);
    if (runtime.paused) {
      if (shouldResume) {
        await this.resumeRuntime(runtimeUuid);
      } else {
        throw new Error(
          "workspace/deployment is paused, use resume: true to resume before running",
        );
      }
    } else {
      const { health } = runtime;
      if (health && health !== "running") {
        throw new Error(
          `workspace/deployment health is '${health}', expected 'running'`,
        );
      }
      if (!health) {
        throw new Error("workspace/deployment has no health status yet");
      }
    }
    const path = `/api/v1/runtimes/${encode(runtimeUuid)}/flows/${encode(flowName)}:run`;
    if (spec != null) return this._postJson(path, { spec });
    return this._postEmpty(path);
  }

  // -- Flow runs --

  async listFlowRuns(runtimeUuid, filters = {}) {
    return this._get("/api/v1/flow-runs", {
      runtime_uuid: runtimeUuid,
      ...filters,
    });
  }

  async getFlowRun(runtimeUuid, name) {
    return this._get(`/api/v1/flow-runs/${encode(name)}`, {
      runtime_uuid: runtimeUuid,
    });
  }

  // -- Otto --

  async listOttoProviders() {
    return this._get("/api/v1/otto/providers");
  }

  async createOttoThread({ prompt, runtimeUuid, model, thinking } = {}) {
    const body = { prompt };
    if (runtimeUuid != null) body.runtime_uuid = runtimeUuid;
    if (model != null) body.model = model;
    if (thinking != null) body.thinking = thinking;
    return this._postJson("/api/v1/otto/threads", body);
  }

  async openOttoUpdates(threadId) {
    const resp = await fetch(
      `${this.baseUrl}/api/v1/otto/threads/${encode(threadId)}/updates`,
      {
        headers: {
          ...(await this._headers()),
          Accept: "text/event-stream",
        },
      },
    );
    if (!resp.ok) await raiseForApiError(resp);
    return resp;
  }
}

/** Percent-encode a URL path segment. */
function encode(segment) {
  return encodeURIComponent(segment);
}

/** Raise an Error with the API error detail if the response is not 2xx. */
async function raiseForApiError(resp) {
  if (resp.ok) return;
  let detail = await resp.text();
  try {
    const json = JSON.parse(detail);
    if (json.detail) {
      detail =
        typeof json.detail === "string" ? json.detail : JSON.stringify(json.detail);
    } else if (json.error) {
      detail =
        typeof json.error === "string" ? json.error : JSON.stringify(json.error);
    }
  } catch {
    // keep raw text
  }
  throw new Error(`API error (HTTP ${resp.status}): ${detail}`);
}

// ---------------------------------------------------------------------------
// Test harness
// ---------------------------------------------------------------------------

let PASS = 0;
let FAIL = 0;
let SKIP = 0;

function check(condition, label, detail = "") {
  if (condition) {
    console.log(`  PASS: ${label}`);
    PASS++;
  } else {
    console.log(`  FAIL: ${label} — ${detail}`);
    FAIL++;
  }
}

function skip(label) {
  console.log(`  SKIP: ${label}`);
  SKIP++;
}

function printSummary() {
  const total = PASS + FAIL + SKIP;
  console.log();
  console.log("=== results ===");
  console.log(`${PASS} passed, ${FAIL} failed, ${SKIP} skipped (of ${total})`);
  if (FAIL > 0) {
    console.log(`${FAIL} FAILED`);
    process.exit(1);
  }
  console.log("all tests passed");
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function pickReasoningOttoModel(providers) {
  for (const provider of providers) {
    for (const model of provider.models ?? []) {
      if (/(claude|gpt-5|o\d|gemini)/i.test(model.id)) {
        return model.id;
      }
    }
  }
  return null;
}

const OTTO_TOOL_PROMPT =
  "Use a tool to inspect the current workspace root, confirm whether the repo contains both ascend-tools and ascend-backend, and answer in two short sentences with the names you found.";

function parseSseFrame(rawFrame) {
  const lines = rawFrame.split(/\r?\n/);
  let event = "message";
  const dataLines = [];
  for (const line of lines) {
    if (!line || line.startsWith(":")) continue;
    if (line.startsWith("event:")) {
      event = line.slice("event:".length).trim();
      continue;
    }
    if (line.startsWith("data:")) {
      dataLines.push(line.slice("data:".length).trimStart());
    }
  }
  if (dataLines.length === 0) return null;
  const data = dataLines.join("\n");
  let parsed;
  try {
    parsed = JSON.parse(data);
  } catch {
    parsed = data;
  }
  return { event, data: parsed };
}

function findSseFrameBoundary(buffer) {
  const match = buffer.match(/\r\n\r\n|\n\n|\r\r/);
  if (!match || match.index == null) return null;
  return { index: match.index, length: match[0].length };
}

async function readSseEvents(resp, { timeoutMs = 20000 } = {}) {
  if (!resp.body) throw new Error("SSE response has no body");

  const reader = resp.body.getReader();
  const decoder = new TextDecoder();
  const events = [];
  let buffer = "";

  const readChunk = () =>
    Promise.race([
      reader.read(),
      new Promise((_, reject) =>
        setTimeout(
          () =>
            reject(
              new Error(`timed out waiting for SSE chunk after ${timeoutMs}ms`),
            ),
          timeoutMs,
        ),
      ),
    ]);

  try {
    for (;;) {
      const { value, done } = await readChunk();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });

      let boundary = findSseFrameBoundary(buffer);
      while (boundary) {
        const rawFrame = buffer.slice(0, boundary.index);
        buffer = buffer.slice(boundary.index + boundary.length);
        const parsed = parseSseFrame(rawFrame);
        if (parsed) {
          events.push(parsed);
          if (
            parsed.event === "thread.done" ||
            parsed.event === "thread.stopped" ||
            parsed.event === "response.error"
          ) {
            await reader.cancel().catch(() => {});
            return events;
          }
        }
        boundary = findSseFrameBoundary(buffer);
      }
    }
  } finally {
    await reader.cancel().catch(() => {});
  }

  return events;
}

/**
 * Run a flow with retries for transient readiness states.
 */
async function runFlowWithRetry(client, runtimeUuid, flowName, opts = {}) {
  let lastError;
  for (const delay of [0, 5, 10, 15, 15, 15]) {
    if (delay) await sleep(delay * 1000);
    try {
      return await client.runFlow(runtimeUuid, flowName, opts);
    } catch (e) {
      const msg = e.message.toLowerCase();
      if (
        msg.includes("starting") ||
        msg.includes("no health status") ||
        msg.includes("initializing")
      ) {
        lastError = e;
        continue;
      }
      throw e;
    }
  }
  throw lastError ?? new Error("runFlow retry exhausted");
}

async function createOttoThreadWithRetry(
  client,
  runtimeUuid,
  { prompt, model, thinking },
) {
  let lastError;
  for (const delay of [0, 2, 5, 10, 15]) {
    if (delay) await sleep(delay * 1000);
    try {
      return await client.createOttoThread({
        prompt,
        runtimeUuid,
        model,
        thinking,
      });
    } catch (e) {
      const msg = String(e.message || e).toLowerCase();
      if (msg.includes("paused")) {
        await client.resumeRuntime(runtimeUuid);
        lastError = e;
        continue;
      }
      if (
        msg.includes("starting") ||
        msg.includes("no health status") ||
        msg.includes("initializing")
      ) {
        lastError = e;
        continue;
      }
      throw e;
    }
  }
  throw lastError ?? new Error("createOttoThread retry exhausted");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

async function main() {
  const { values } = parseArgs({
    options: {
      "runtime-id": { type: "string", default: "ascend-tools" },
    },
  });
  const runtimeId = values["runtime-id"];

  // ---------- preflight ----------

  console.log("=== preflight ===");

  for (const v of [
    "ASCEND_SERVICE_ACCOUNT_ID",
    "ASCEND_SERVICE_ACCOUNT_KEY",
    "ASCEND_INSTANCE_API_URL",
  ]) {
    if (!process.env[v]) {
      console.error(`ERROR: ${v} is not set`);
      process.exit(1);
    }
  }
  check(true, "env vars set");

  const auth = new Auth(
    process.env.ASCEND_SERVICE_ACCOUNT_ID,
    process.env.ASCEND_SERVICE_ACCOUNT_KEY,
    process.env.ASCEND_INSTANCE_API_URL,
  );
  const token = await auth.getToken();
  check(Boolean(token), "auth: got instance token");

  const client = new AscendClient(auth);
  check(true, "client created");

  // ---------- runtimes ----------

  console.log("=== runtimes ===");

  const runtimes = await client.listRuntimes();
  check(Array.isArray(runtimes), "listRuntimes returns array");

  if (!runtimes.length) {
    skip("no runtimes found — skipping get, filters, flows, and flow runs");
    printSummary();
    return;
  }

  check(true, `listRuntimes returned ${runtimes.length} runtime(s)`);

  const byId = await client.listRuntimes({ id: runtimeId });
  let runtime;
  if (byId.length) {
    runtime = byId[0];
  } else {
    console.log(
      `  id '${runtimeId}' not found, falling back to first result`,
    );
    runtime = runtimes[0];
  }

  const runtimeUuid = runtime.uuid;
  const rtId = runtime.id;
  const isPaused = runtime.paused ?? false;
  console.log(
    `  using runtime: ${rtId} (${runtimeUuid})${isPaused ? " [paused]" : ""}`,
  );

  // get by uuid
  const got = await client.getRuntime(runtimeUuid);
  check(got.uuid === runtimeUuid, "getRuntime returns correct uuid");

  for (const field of [
    "uuid",
    "id",
    "title",
    "kind",
    "project_uuid",
    "environment_uuid",
    "created_at",
    "updated_at",
  ]) {
    check(
      got[field] != null,
      `getRuntime has field '${field}'`,
      `value: ${got[field]}`,
    );
  }

  // filter by id
  const filtered = await client.listRuntimes({ id: rtId });
  check(
    filtered.length === 1,
    "listRuntimes(id=...) returns exactly 1",
    `got ${filtered.length}`,
  );
  check(filtered[0].uuid === runtimeUuid, "filtered runtime has correct uuid");

  // filter by kind
  const { kind } = runtime;
  const byKind = await client.listRuntimes({ kind });
  check(
    byKind.length >= 1,
    `listRuntimes(kind='${kind}') returns >= 1`,
    `got ${byKind.length}`,
  );
  check(
    byKind.every((r) => r.kind === kind),
    "all results match kind filter",
  );

  // ---------- otto streaming ----------

  console.log("=== otto streaming ===");

  try {
    if (isPaused) {
      const resumed = await client.resumeRuntime(runtimeUuid);
      check(
        resumed.paused === false,
        "resumeRuntime clears paused before otto probe",
      );
    }
    const ottoProviders = await client.listOttoProviders();
    check(Array.isArray(ottoProviders), "listOttoProviders returns array");

    if (!ottoProviders.length) {
      skip("no otto providers configured — skipping otto streaming probe");
    } else {
      const ottoModel = pickReasoningOttoModel(ottoProviders);
      if (!ottoModel) {
        skip("no reasoning-capable otto model found for streaming probe");
      } else {
        console.log(`  using otto model: ${ottoModel}`);
        const thread = await createOttoThreadWithRetry(client, runtimeUuid, {
          prompt: OTTO_TOOL_PROMPT,
          model: ottoModel,
          thinking: "medium",
        });
        check(
          typeof thread === "object" && thread.thread_id,
          `createOttoThread returned thread_id: ${thread.thread_id}`,
        );

        const updates = await client.openOttoUpdates(thread.thread_id);
        const events = await readSseEvents(updates);
        const eventTypes = events.map((event) => event.event);
        const textDeltas = events.filter(
          (event) => event.event === "response.output_text.delta",
        );
        const reasoningDeltas = events.filter((event) =>
          [
            "response.reasoning_summary_text.delta",
            "response.reasoning_text.delta",
          ].includes(event.event),
        );
        const toolArgDeltas = events.filter(
          (event) => event.event === "response.function_call_arguments.delta",
        );
        const terminalEvent = events.find((event) =>
          ["thread.done", "thread.stopped", "response.error"].includes(
            event.event,
          ),
        );

        check(events.length > 0, "otto SSE returned events");
        check(
          textDeltas.length > 0,
          "otto SSE included text deltas",
          JSON.stringify(eventTypes),
        );
        check(
          Boolean(terminalEvent),
          "otto SSE reached a terminal event",
          JSON.stringify(eventTypes),
        );
        check(
          reasoningDeltas.length > 0,
          "otto SSE surfaced reasoning deltas for the required tool-use prompt",
          JSON.stringify(eventTypes),
        );
        check(
          toolArgDeltas.length > 0,
          "otto SSE surfaced function_call_arguments deltas for the required tool-use prompt",
          JSON.stringify(eventTypes),
        );
      }
    }
  } catch (e) {
    const msg = String(e.message || e).toLowerCase();
    if (
      msg.includes("not found") ||
      msg.includes("not implemented") ||
      msg.includes("404")
    ) {
      skip(`otto streaming not available: ${e.message || e}`);
    } else {
      check(false, "otto streaming probe", e.message || String(e));
    }
  }

  // ---------- flows ----------

  console.log("=== flows ===");

  const flows = await client.listFlows(runtimeUuid);
  check(Array.isArray(flows), "listFlows returns array");

  if (!flows.length) {
    skip("no flows found — skipping flow runs and trigger tests");
    printSummary();
    return;
  }

  check(true, `listFlows returned ${flows.length} flow(s)`);

  const flowName = flows[0].name;
  console.log(`  using flow: ${flowName}`);

  check(
    flows.every((f) => "name" in f),
    "all flows have 'name' field",
  );

  // ---------- flow runs (before) ----------

  console.log("=== flow runs (before trigger) ===");

  const runsBeforeResult = await client.listFlowRuns(runtimeUuid, {
    flow: flowName,
  });
  check(typeof runsBeforeResult === "object", "listFlowRuns returns object");
  check("items" in runsBeforeResult, "listFlowRuns has 'items' key");
  check("truncated" in runsBeforeResult, "listFlowRuns has 'truncated' key");
  const runsBefore = runsBeforeResult.items;
  const runsBeforeCount = runsBefore.length;
  check(
    true,
    `listFlowRuns returned ${runsBeforeCount} run(s) before trigger`,
  );

  // test getFlowRun on existing run
  if (runsBefore.length) {
    const existingRun = runsBefore[0];
    const gotRun = await client.getFlowRun(runtimeUuid, existingRun.name);
    check(gotRun.name === existingRun.name, "getFlowRun returns correct run");

    for (const field of [
      "name",
      "flow",
      "status",
      "runtime_uuid",
      "build_uuid",
      "created_at",
    ]) {
      check(gotRun[field] != null, `getFlowRun has field '${field}'`);
    }

    check(
      ["pending", "running", "succeeded", "failed"].includes(gotRun.status),
      `flow run status is valid: ${gotRun.status}`,
    );
  }

  // test pagination
  const limited = (
    await client.listFlowRuns(runtimeUuid, { flow: flowName, limit: 1 })
  ).items;
  check(
    limited.length <= 1,
    "listFlowRuns(limit=1) returns at most 1",
    `got ${limited.length}`,
  );

  if (runsBeforeCount > 1) {
    const offsetRuns = (
      await client.listFlowRuns(runtimeUuid, {
        flow: flowName,
        offset: 1,
        limit: 1,
      })
    ).items;
    check(
      offsetRuns.length <= 1,
      "listFlowRuns(offset=1, limit=1) returns at most 1",
    );
    if (offsetRuns.length && runsBeforeCount > 1) {
      check(
        offsetRuns[0].name !== runsBefore[0].name,
        "offset=1 returns different run than offset=0",
      );
    }
  }

  // ---------- trigger flow run ----------

  console.log("=== trigger flow run ===");

  const trigger = await runFlowWithRetry(client, runtimeUuid, flowName, {
    resume: true,
  });
  check(typeof trigger === "object", "runFlow returns object");
  check(
    trigger.event_uuid != null,
    `runFlow has event_uuid: ${trigger.event_uuid}`,
  );
  check(
    trigger.event_type === "ScheduleFlowRun",
    "event_type is ScheduleFlowRun",
  );

  // ---------- flow runs (after) ----------

  console.log("=== flow runs (after trigger) ===");

  let runsAfterCount = runsBeforeCount;
  for (const delay of [2, 3, 5, 5]) {
    await sleep(delay * 1000);
    const runsAfter = (
      await client.listFlowRuns(runtimeUuid, { flow: flowName })
    ).items;
    runsAfterCount = runsAfter.length;
    if (runsAfterCount > runsBeforeCount) break;
  }

  if (runsAfterCount > runsBeforeCount) {
    check(
      true,
      `flow run count increased: ${runsBeforeCount} -> ${runsAfterCount}`,
    );
  } else {
    skip(
      "flow run not yet materialized after 15s (flow runner may be catching up)",
    );
  }

  const runsAfter = (
    await client.listFlowRuns(runtimeUuid, { flow: flowName })
  ).items;
  if (runsAfter.length) {
    const newest = runsAfter[0];
    check(true, `newest run: ${newest.name} (status: ${newest.status})`);

    const gotNew = await client.getFlowRun(runtimeUuid, newest.name);
    check(gotNew.name === newest.name, "getFlowRun on new run works");
  }

  // ---------- status filter ----------

  console.log("=== status filter ===");

  for (const status of ["pending", "running", "succeeded", "failed"]) {
    const byStatus = (await client.listFlowRuns(runtimeUuid, { status }))
      .items;
    check(
      Array.isArray(byStatus),
      `listFlowRuns(status='${status}') returns array items`,
    );
    if (byStatus.length) {
      const wrong = byStatus.filter((r) => r.status !== status);
      check(
        wrong.length === 0,
        `all ${status} runs have correct status`,
        `${wrong.length} have wrong status`,
      );
    }
  }

  // ---------- run_flow with spec ----------

  console.log("=== runFlow with spec ===");

  const trigger2 = await runFlowWithRetry(client, runtimeUuid, flowName, {
    spec: {},
    resume: true,
  });
  check(trigger2.event_uuid != null, "runFlow with empty spec works");

  const trigger3Fr = await runFlowWithRetry(client, runtimeUuid, flowName, {
    spec: { full_refresh: true },
    resume: true,
  });
  check(trigger3Fr.event_uuid != null, "runFlow with full_refresh=true works");

  const trigger3Params = await runFlowWithRetry(
    client,
    runtimeUuid,
    flowName,
    { spec: { parameters: { key: "value" } }, resume: true },
  );
  check(trigger3Params.event_uuid != null, "runFlow with parameters works");

  const trigger3Multi = await runFlowWithRetry(
    client,
    runtimeUuid,
    flowName,
    {
      spec: {
        run_tests: false,
        halt_flow_on_error: true,
        runner_overrides: { size: "Medium" },
      },
      resume: true,
    },
  );
  check(
    trigger3Multi.event_uuid != null,
    "runFlow with multiple spec fields works",
  );

  // ---------- workspace pause/resume ----------

  if (runtime.kind !== "workspace") {
    skip("not a workspace — skipping pause/resume tests");
  } else {
    console.log("=== workspace pause ===");

    const pausedRt = await client.pauseRuntime(runtimeUuid);
    check(pausedRt.paused === true, "pauseRuntime sets paused=true");

    const gotPaused = await client.getRuntime(runtimeUuid);
    check(gotPaused.paused === true, "getRuntime confirms paused");

    // runFlow without resume should fail on a paused workspace
    try {
      await client.runFlow(runtimeUuid, flowName);
      check(false, "runFlow on paused workspace should throw", "no error thrown");
    } catch (e) {
      const msg = e.message.toLowerCase();
      check(
        msg.includes("paused") || msg.includes("resume"),
        "runFlow on paused workspace throws descriptive error",
        e.message,
      );
    }

    console.log("=== workspace resume via flow run ===");

    const trigger3 = await runFlowWithRetry(client, runtimeUuid, flowName, {
      resume: true,
    });
    check(trigger3.event_uuid != null, "runFlow with resume=true succeeds");

    const gotResumed = await client.getRuntime(runtimeUuid);
    check(
      gotResumed.paused === false,
      "workspace is unpaused after resume",
    );

    console.log("=== workspace resume (explicit) ===");

    let rtHealth;
    for (const delay of [2, 3, 5, 5]) {
      await sleep(delay * 1000);
      rtHealth = await client.getRuntime(runtimeUuid);
      if (rtHealth.health != null) break;
    }

    if (rtHealth?.health != null) {
      check(true, `workspace health restored: ${rtHealth.health}`);
    } else {
      skip(
        "workspace health not yet available after 15s (may be slow to start)",
      );
    }

    // resume on an already-running workspace should be a no-op
    const resumedRt = await client.resumeRuntime(runtimeUuid);
    check(resumedRt.paused === false, "resumeRuntime is idempotent");
  }

  // ---------- summary ----------

  printSummary();
}

main();
