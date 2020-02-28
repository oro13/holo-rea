const {
  getDNA,
  buildConfig,
  buildRunner,
  buildPlayer,
} = require('../init')

const runner = buildRunner()

const config = buildConfig({
  proposal: getDNA('proposal'),
  planning: getDNA('planning'),
  agent: getDNA('agent'),
}, {})

const exampleProposal = {
  name: 'String',
  hasBeginning: '2019-11-19T00:00:00.056Z',
  hasEnd: '2019-11-19T00:00:00.056Z',
  inScopeOf: null,
  unitBased: true,
  created: '2019-11-19T00:00:00.056Z',
  note: 'note'
}

const exampleIntent = {
  action: 'move',
}

const grepme = obj => console.log('grepme: ', JSON.stringify(obj))

runner.registerScenario('ProposedIntent record API', async (s, t) => {
  const alice = await buildPlayer(s, 'alice', config)
  let proposalRes = await alice.graphQL(`
    mutation($rs: ProposalCreateParams!) {
      res: createProposal(proposal: $rs) {
        proposal {
          id
        }
      }
    }
  `, {
    rs: exampleProposal,
  })

  let proposalID = proposalRes.data.res.proposal.id

  await s.consistency()

  let createResp = await alice.graphQL(`
    mutation($pIn: ID!, $ps: ID!, $re: Boolean) {
      res: proposeIntent(publishedIn: $pIn, publishes: $ps, reciprocal: $re) {
        proposedIntent {
          id
        }
      }
    }
  `, {
    pIn: proposalID, // Proposal Address
    ps: 'fgrfrfgrgrgrgrtgtgdtgdtrgt', // Intent Address
    re: true,
  })
  await s.consistency()
  grepme(createResp)
  t.ok(createResp.data.res.proposedIntent.id, 'record created')

  const psID = createResp.data.res.proposedIntent.id

  let getResp = await alice.graphQL(`
    query($id: ID!) {
      res: proposal(id: $id) {
        id
        publishes {
          id
        }
      }
    }
  `, {
    id: proposalID,
  })
  grepme(getResp.data.res)
  grepme({ id: proposalID, publishes: [{ id: psID }] })
  t.deepEqual(getResp.data.res, { id: proposalID, publishes: [{ id: psID }] }, 'record read OK')

  const deleteResult = await alice.graphQL(`
    mutation($id: String!) {
      res: deleteProposal(id: $id)
    }
  `, {
    id: psID,
  })
  await s.consistency()

  t.equal(deleteResult.data.res, true)

  const queryForDeleted = await alice.graphQL(`
    query($id: ID!) {
      res: proposal(id: $id) {
        id
      }
    }
  `, {
    id: psID,
  })
  t.equal(queryForDeleted.errors.length, 1, 'querying deleted record is an error')
  t.notEqual(-1, queryForDeleted.errors[0].message.indexOf('No entry at this address'), 'correct error reported')
})

runner.registerScenario('ProposedIntent external link', async (s, t) => {
  const alice = await buildPlayer(s, 'alice', config)
  /*
  * the next code is only for getting an intent and a proposal to link to the proposedIntent.
  * the idea is to verify the intent linking by getting Proposal->ProposedIntent->Intent
  */
  const agentAddress = (await alice.graphQL(`{
    myAgent {
      id
    }
  }`)).data.myAgent.id

  exampleIntent.provider = agentAddress

  // intent creation
  let intentRes = await alice.graphQL(`
    mutation($rs: IntentCreateParams!) {
      res: createIntent(intent: $rs) {
        intent {
          id
        }
      }
    }
  `, {
    rs: exampleIntent,
  })
  const intentAdress = intentRes.data.res.intent.id

  t.ok(intentAdress, 'can create intent')

  // proposal creation
  let proposalRes = await alice.graphQL(`
    mutation($rs: ProposalCreateParams!) {
      res: createProposal(proposal: $rs) {
        proposal {
          id
        }
      }
    }
  `, {
    rs: exampleProposal,
  })

  let proposalAdress = proposalRes.data.res.proposal.id

  t.ok(proposalAdress, 'can create proposal')

  await s.consistency()

  let proposeIntentResp = await alice.graphQL(`
    mutation($pIn: ID!, $ps: ID!, $re: Boolean) {
      res: proposeIntent(publishedIn: $pIn, publishes: $ps, reciprocal: $re) {
        proposedIntent {
          id
        }
      }
    }
  `, {
    pIn: proposalAdress, // Proposal Address
    ps: intentAdress, // Intent Address
    re: true,
  })

  t.ok(proposeIntentResp.data.res.proposedIntent.id, 'can propose')

  const proposedIntentAdress = proposeIntentResp.data.res.proposedIntent.id

  await s.consistency()

  let getResp = await alice.graphQL(`
    query($id: ID!) {
      res: proposal(id: $id) {
        id
        publishes {
          id
          publishes{
            id
          }
        }
      }
    }
  `, {
    id: proposalAdress,
  })
  grepme(getResp)
  t.deepEqual(getResp, { data: { res: { id: proposalAdress, publishes: [{ id: proposedIntentAdress, publishes: { id: intentAdress } }] } } }, 'Nested fetching')
})

runner.run()
