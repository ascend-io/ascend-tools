import { Client } from '@ascend-io/ascend-tools'
import express from 'express'
import { fileURLToPath } from 'url'
import { dirname, join } from 'path'

const __dirname = dirname(fileURLToPath(import.meta.url))
const app = express()
const client = new Client()
const port = process.env.PORT || 3000

app.use(express.json())
app.use(express.static(join(__dirname, 'public')))

// --- HTML helpers ---

function escapeHtml(str) {
  if (str == null) return ''
  return String(str).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')
}

function statusBadge(health) {
  const colors = {
    running: 'green', healthy: 'green',
    paused: 'orange', stopped: 'orange',
    error: 'red', failed: 'red',
    starting: 'blue', pending: 'blue',
  }
  const color = colors[String(health).toLowerCase()] || 'gray'
  return `<span style="color:${color};font-weight:bold">${escapeHtml(health || 'unknown')}</span>`
}

function detailTable(obj) {
  const rows = Object.entries(obj)
    .filter(([, v]) => v != null && v !== '')
    .map(([k, v]) => `<tr><th>${escapeHtml(k)}</th><td>${escapeHtml(typeof v === 'object' ? JSON.stringify(v) : v)}</td></tr>`)
    .join('')
  return `<table role="grid">${rows}</table>`
}

// --- API routes returning HTML fragments ---

app.get('/api/workspaces', async (req, res) => {
  try {
    const workspaces = await client.listWorkspaces()
    if (!workspaces.length) return res.send('<p>No workspaces found.</p>')
    const rows = workspaces.map(ws => `
      <tr data-clickable hx-get="/api/workspace/${encodeURIComponent(ws.title)}" hx-target="#detail" hx-swap="innerHTML">
        <td>${escapeHtml(ws.title)}</td>
        <td>${statusBadge(ws.health)}</td>
        <td>${escapeHtml(ws.environmentUuid)}</td>
        <td>${escapeHtml(ws.profileName)}</td>
      </tr>`).join('')
    res.send(`
      <table role="grid">
        <thead><tr><th>Title</th><th>Health</th><th>Environment</th><th>Profile</th></tr></thead>
        <tbody>${rows}</tbody>
      </table>`)
  } catch (e) {
    res.send(`<p class="error">Error: ${escapeHtml(e.message)}</p>`)
  }
})

app.get('/api/workspace/:title', async (req, res) => {
  try {
    const ws = await client.getWorkspace(req.params.title)
    res.send(`
      <h3>${escapeHtml(ws.title)}</h3>
      ${detailTable(ws)}
      <div hx-get="/api/flows?workspace=${encodeURIComponent(ws.title)}" hx-trigger="load" hx-swap="innerHTML"></div>`)
  } catch (e) {
    res.send(`<p class="error">Error: ${escapeHtml(e.message)}</p>`)
  }
})

app.get('/api/deployments', async (req, res) => {
  try {
    const deployments = await client.listDeployments()
    if (!deployments.length) return res.send('<p>No deployments found.</p>')
    const rows = deployments.map(d => `
      <tr data-clickable hx-get="/api/deployment/${encodeURIComponent(d.title)}" hx-target="#detail" hx-swap="innerHTML">
        <td>${escapeHtml(d.title)}</td>
        <td>${statusBadge(d.health)}</td>
        <td>${escapeHtml(d.environmentUuid)}</td>
        <td>${escapeHtml(d.profileName)}</td>
      </tr>`).join('')
    res.send(`
      <table role="grid">
        <thead><tr><th>Title</th><th>Health</th><th>Environment</th><th>Profile</th></tr></thead>
        <tbody>${rows}</tbody>
      </table>`)
  } catch (e) {
    res.send(`<p class="error">Error: ${escapeHtml(e.message)}</p>`)
  }
})

app.get('/api/deployment/:title', async (req, res) => {
  try {
    const d = await client.getDeployment(req.params.title)
    res.send(`
      <h3>${escapeHtml(d.title)}</h3>
      ${detailTable(d)}
      <div hx-get="/api/flows?deployment=${encodeURIComponent(d.title)}" hx-trigger="load" hx-swap="innerHTML"></div>`)
  } catch (e) {
    res.send(`<p class="error">Error: ${escapeHtml(e.message)}</p>`)
  }
})

app.get('/api/environments', async (req, res) => {
  try {
    const envs = await client.listEnvironments()
    if (!envs.length) return res.send('<p>No environments found.</p>')
    const rows = envs.map(e => `<tr><td>${escapeHtml(e.title)}</td><td>${escapeHtml(e.id)}</td><td>${escapeHtml(e.uuid)}</td></tr>`).join('')
    res.send(`
      <table role="grid">
        <thead><tr><th>Title</th><th>ID</th><th>UUID</th></tr></thead>
        <tbody>${rows}</tbody>
      </table>`)
  } catch (e) {
    res.send(`<p class="error">Error: ${escapeHtml(e.message)}</p>`)
  }
})

app.get('/api/projects', async (req, res) => {
  try {
    const projects = await client.listProjects()
    if (!projects.length) return res.send('<p>No projects found.</p>')
    const rows = projects.map(p => `<tr><td>${escapeHtml(p.title)}</td><td>${escapeHtml(p.id)}</td><td>${escapeHtml(p.path)}</td></tr>`).join('')
    res.send(`
      <table role="grid">
        <thead><tr><th>Title</th><th>ID</th><th>Path</th></tr></thead>
        <tbody>${rows}</tbody>
      </table>`)
  } catch (e) {
    res.send(`<p class="error">Error: ${escapeHtml(e.message)}</p>`)
  }
})

app.get('/api/flows', async (req, res) => {
  try {
    const { workspace, deployment } = req.query
    const flows = await client.listFlows(workspace || null, deployment || null)
    if (!flows.length) return res.send('<p>No flows found.</p>')
    const target = workspace || deployment
    const targetParam = workspace ? 'workspace' : 'deployment'
    const rows = flows.map(f => `
      <tr>
        <td>${escapeHtml(f.name)}</td>
        <td>
          <button hx-post="/api/flow/${encodeURIComponent(f.name)}/run?${targetParam}=${encodeURIComponent(target)}"
                  hx-target="#flow-status" hx-swap="innerHTML">
            Run
          </button>
        </td>
      </tr>`).join('')
    res.send(`
      <h4>Flows</h4>
      <table role="grid">
        <thead><tr><th>Name</th><th>Action</th></tr></thead>
        <tbody>${rows}</tbody>
      </table>
      <div id="flow-status"></div>`)
  } catch (e) {
    res.send(`<p class="error">Error: ${escapeHtml(e.message)}</p>`)
  }
})

app.post('/api/flow/:name/run', async (req, res) => {
  try {
    const { workspace, deployment } = req.query
    const result = await client.runFlow(req.params.name, workspace || null, deployment || null)
    const targetParam = workspace ? `workspace=${encodeURIComponent(workspace)}` : `deployment=${encodeURIComponent(deployment)}`
    res.send(`
      <p>Flow triggered: ${escapeHtml(result.eventUuid)}</p>
      <div hx-get="/api/flow-runs?${targetParam}&flow_name=${encodeURIComponent(req.params.name)}&limit=1"
           hx-trigger="load delay:2s, every 5s" hx-swap="innerHTML"></div>`)
  } catch (e) {
    res.send(`<p class="error">Error: ${escapeHtml(e.message)}</p>`)
  }
})

app.get('/api/flow-runs', async (req, res) => {
  try {
    const { workspace, deployment, flow_name, status, limit } = req.query
    const result = await client.listFlowRuns(
      workspace || null, deployment || null, null,
      status || null, flow_name || null, null, null, null,
      limit ? parseInt(limit) : null,
    )
    const runs = result.items || []
    if (!runs.length) return res.send('<p>No flow runs found.</p>')
    const rows = runs.map(r => `
      <tr data-clickable hx-get="/api/flow-run/${encodeURIComponent(r.name)}?${workspace ? `workspace=${encodeURIComponent(workspace)}` : `deployment=${encodeURIComponent(deployment)}`}" hx-target="#detail" hx-swap="innerHTML">
        <td>${escapeHtml(r.name)}</td>
        <td>${escapeHtml(r.flow)}</td>
        <td>${statusBadge(r.status)}</td>
        <td>${escapeHtml(r.createdAt)}</td>
      </tr>`).join('')
    res.send(`
      <table role="grid">
        <thead><tr><th>Name</th><th>Flow</th><th>Status</th><th>Created</th></tr></thead>
        <tbody>${rows}</tbody>
      </table>`)
  } catch (e) {
    res.send(`<p class="error">Error: ${escapeHtml(e.message)}</p>`)
  }
})

app.get('/api/flow-run/:name', async (req, res) => {
  try {
    const { workspace, deployment } = req.query
    const run = await client.getFlowRun(req.params.name, workspace || null, deployment || null)
    const terminal = ['succeeded', 'failed', 'cancelled'].includes(String(run.status).toLowerCase())
    const poll = terminal ? '' : `hx-get="/api/flow-run/${encodeURIComponent(run.name)}?${workspace ? `workspace=${encodeURIComponent(workspace)}` : `deployment=${encodeURIComponent(deployment)}`}" hx-trigger="every 5s" hx-swap="outerHTML"`
    res.send(`
      <div ${poll}>
        <h3>Flow Run: ${escapeHtml(run.name)}</h3>
        ${detailTable(run)}
      </div>`)
  } catch (e) {
    res.send(`<p class="error">Error: ${escapeHtml(e.message)}</p>`)
  }
})

// --- Otto routes ---

app.get('/api/otto/providers', async (req, res) => {
  try {
    const providers = await client.listOttoProviders()
    res.json(providers)
  } catch (e) {
    res.status(500).json({ error: e.message })
  }
})

app.get('/api/otto/workspaces', async (req, res) => {
  try {
    const workspaces = await client.listWorkspaces()
    res.json(workspaces.map(ws => ({ title: ws.title, uuid: ws.uuid })))
  } catch (e) {
    res.status(500).json({ error: e.message })
  }
})

app.post('/api/otto/chat', async (req, res) => {
  const { prompt, workspace, threadId, provider, model } = req.body

  res.writeHead(200, {
    'Content-Type': 'text/event-stream',
    'Cache-Control': 'no-cache',
    'Connection': 'keep-alive',
  })

  try {
    const result = await client.ottoChatStreaming(
      prompt,
      (err, delta) => {
        if (err) {
          res.write(`event: error\ndata: ${JSON.stringify(err.message)}\n\n`)
          return
        }
        res.write(`event: delta\ndata: ${JSON.stringify(delta)}\n\n`)
      },
      workspace || null,  // workspace
      null,               // deployment
      null,               // uuid
      threadId || null,    // thread_id
      model || null,       // model
      provider || null,    // provider
    )
    res.write(`event: done\ndata: ${JSON.stringify({ thread_id: result.threadId })}\n\n`)
    res.end()
  } catch (e) {
    res.write(`event: error\ndata: ${JSON.stringify(e.message)}\n\n`)
    res.end()
  }
})

app.listen(port, () => {
  console.log(`Ascend Tools demo app running at http://localhost:${port}`)
})
