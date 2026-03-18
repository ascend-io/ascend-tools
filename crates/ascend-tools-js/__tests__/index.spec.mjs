import test from 'ava'
import { Client } from '../index.js'

test('Client constructor exists', (t) => {
  t.is(typeof Client, 'function')
})

test('Client constructor succeeds with env vars or throws without', (t) => {
  const hasEnv = process.env.ASCEND_SERVICE_ACCOUNT_ID
    && process.env.ASCEND_SERVICE_ACCOUNT_KEY
    && process.env.ASCEND_INSTANCE_API_URL

  if (hasEnv) {
    const client = new Client()
    t.truthy(client)
  } else {
    t.throws(() => new Client(), {
      message: /ASCEND_SERVICE_ACCOUNT_ID|ASCEND_SERVICE_ACCOUNT_KEY|ASCEND_INSTANCE_API_URL/,
    })
  }
})

test('Client has all expected methods', (t) => {
  const methods = [
    'listWorkspaces', 'getWorkspace', 'pauseWorkspace', 'resumeWorkspace',
    'createWorkspace', 'updateWorkspace', 'deleteWorkspace',
    'listDeployments', 'getDeployment', 'createDeployment', 'updateDeployment',
    'pauseDeploymentAutomations', 'resumeDeploymentAutomations', 'deleteDeployment',
    'listEnvironments', 'getEnvironment',
    'listProjects', 'getProject',
    'listProfiles',
    'listFlows', 'runFlow',
    'listFlowRuns', 'getFlowRun',
    'listOttoProviders', 'otto', 'ottoStreaming',
  ]
  for (const method of methods) {
    t.is(typeof Client.prototype[method], 'function', `Client.prototype.${method} should be a function`)
  }
})
