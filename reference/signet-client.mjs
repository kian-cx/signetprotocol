#!/usr/bin/env node
/**
 * signet-client — turns ANY Claude Code session into an autonomous agent on the
 * Signet network. Non-blocking (each command exits), so a coding session can:
 *   1. `poll`  → get its task inbox from the Relay (signed, verified identity)
 *   2. do the work with its own tools (Xcode/simulator, editing, building…)
 *   3. `send`  → report the result back
 * and loop. Keyed Ed25519 identity persisted per handle → messages are verified.
 *
 *   node signet-client.mjs poll  --handle ios-worker-1 --owner kian --base https://<tu-relay>
 *   node signet-client.mjs send  --handle ios-worker-1 --to kian --body "listo: compilé y probé X" --base ...
 */
import { generateKeyPairSync, sign as edSign, createPrivateKey } from "node:crypto";
import { readFileSync, writeFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const arg = (n, d) => { const i = process.argv.indexOf(`--${n}`); return i > -1 && process.argv[i + 1] ? process.argv[i + 1] : d; };
const CMD = process.argv[2] || "poll";
const BASE = arg("base", process.env.SIGNET_BASE || "https://<tu-relay>");
const HANDLE = arg("handle", "worker-1");
const OWNER = arg("owner", "kian");
const ID_FILE = join(__dirname, "..", "data", `signet-agent-${HANDLE}.json`);

const SESSION_V = "aether-session-v1", MSG_V = "aether-msg-v1";
const canonical = (v) => v === undefined ? "null" : (v === null || typeof v !== "object") ? JSON.stringify(v)
  : Array.isArray(v) ? `[${v.map(canonical).join(",")}]`
  : `{${Object.keys(v).sort().map((k) => `${JSON.stringify(k)}:${canonical(v[k])}`).join(",")}}`;
const sessionPayload = (h, n) => JSON.stringify([SESSION_V, h, n]);
const msgPayload = (f, t, k, b, c, n) => JSON.stringify([MSG_V, f, t, k, b, c, n, canonical(undefined)]);
const signStr = (priv, d) => edSign(null, Buffer.from(d, "utf8"), priv).toString("base64");

function loadIdentity() {
  if (existsSync(ID_FILE)) { const j = JSON.parse(readFileSync(ID_FILE, "utf8")); return { priv: createPrivateKey(j.private_key_pem), public_key: j.public_key }; }
  const { publicKey, privateKey } = generateKeyPairSync("ed25519");
  const id = { public_key: publicKey.export({ type: "spki", format: "der" }).toString("base64"), private_key_pem: privateKey.export({ type: "pkcs8", format: "pem" }).toString() };
  mkdirSync(dirname(ID_FILE), { recursive: true }); writeFileSync(ID_FILE, JSON.stringify(id, null, 2));
  return { priv: createPrivateKey(id.private_key_pem), public_key: id.public_key };
}
const relay = async (action, fields = {}) => (await fetch(`${BASE}/api/relay`, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ action, ...fields }) })).json();

async function login(id) {
  await relay("register", { handle: HANDLE, public_key: id.public_key, display_name: `${HANDLE} (agent)` });
  const start = await relay("session_start", { handle: HANDLE });
  if (!start.ok) throw new Error("session_start: " + start.error);
  const done = await relay("session_complete", { challenge_id: start.challenge_id, signature: signStr(id.priv, sessionPayload(HANDLE, start.nonce)) });
  if (!done.ok) throw new Error("session_complete: " + done.error);
  return done.token;
}
async function ensureTrust(token) {
  const cx = await relay("contacts", { token });
  if (!(cx.contacts || []).find((c) => c.peer === OWNER)) await relay("contact_request", { token, to_handle: OWNER });
  for (const p of cx.pending_incoming || []) await relay("contact_respond", { token, contact_id: p.id, decision: "accept" });
}

const id = loadIdentity();
const token = await login(id);
await ensureTrust(token);

if (CMD === "poll") {
  const since = arg("since", null);
  const r = await relay("poll", { token, since, limit: 50 });
  const msgs = (r.messages || r.inbox || []).filter((m) => m.from_handle !== HANDLE);
  console.log(JSON.stringify({ ok: true, handle: HANDLE, count: msgs.length, messages: msgs.map((m) => ({ id: m.id, from: m.from_handle, body: m.body, verified: m.verified, at: m.created_at || m.delivered_at })) }, null, 2));
} else if (CMD === "send") {
  const to = arg("to", OWNER), body = arg("body", "");
  const created_at = new Date().toISOString();
  const nonce = `n_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;
  const r = await relay("send", { token, to_handle: to, body, kind: "agent_dm", created_at, nonce, signature: signStr(id.priv, msgPayload(HANDLE, to, "agent_dm", body, created_at, nonce)) });
  console.log(JSON.stringify({ ok: r.ok, to, error: r.error }, null, 2));
} else if (CMD === "contacts") {
  const cx = await relay("contacts", { token });
  console.log(JSON.stringify({ ok: cx.ok, accepted: (cx.contacts || []).map((c) => `${c.peer}:${c.state}`), pending_incoming: (cx.pending_incoming || []).map((p) => p.from_handle), pending_outgoing: (cx.pending_outgoing || []).map((p) => p.to_handle) }, null, 2));
} else if (CMD === "trust") {
  const to = arg("to", "");
  const r = await relay("contact_request", { token, to_handle: to });
  console.log(JSON.stringify({ requested: to, ok: r.ok, error: r.error }, null, 2));
} else if (CMD === "lookup") {
  const who = arg("who", "");
  const r = await relay("lookup", { token, handle: who });
  console.log(JSON.stringify({ who, exists: !!(r.ok && (r.user || r.handle || r.public_key)), r }, null, 2));
} else {
  console.log(`uso: signet-client.mjs poll|send|contacts|trust|lookup --handle H --owner O --base URL`);
}
