const {
  getDNA,
  buildConfig,
  buildRunner,
  buildPlayer,
} = require('../init')

const runner = buildRunner()

const config = buildConfig({
  proposal: getDNA('proposal'),
}, {})

const agentAddress = 'RANDOMAGENADDRESSESDEJOSEJFEOFJESFOI'

const exampleProposal = {
  name: "String",
  hasBeginning: '2019-11-19T00:00:00.056Z',
  hasEnd: '2019-11-19T00:00:00.056Z',
  inScopeOf: null,
  unitBased: true,
  created: '2019-11-19T00:00:00.056Z',
  note: 'note'
}

const grepMe = obj => console.log( 'grepme: ', JSON.stringify( obj ) )

runner.registerScenario('ProposedTo record API', async (s, t) => {
  const alice = await buildPlayer(s, 'alice', config)
  grepMe('start')
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
  grepMe('gr2')

  let proposalID = proposalRes.data.res.proposal.id

  await s.consistency()
  grepMe('gr3')

  let createResp = await alice.graphQL(`
    mutation($p: ID!, $pTo: ID!) {
      res: proposeTo(proposed: $p,proposedTo: $pTo) {
        proposedTo {
          id
        }
      }
    }
  `, {
    p: proposalID,
    pTo: agentAddress,
  })
  grepMe('gr4')
  await s.consistency()
  grepMe('gr5')
  t.ok(createResp.data.res.proposedTo.id, 'record created')

  const psId = createResp.data.res.proposedTo.id

  let getResp = await alice.graphQL(`
    query($id: ID!) {
      res: proposal(id: $id) {
        id
        publishedTo {
          id
        }
      }
    }
  `, {
    id: proposalID,
  })
  grepMe('getResp')
  grepMe(getResp)
  t.ok(getResp.data.res.publishedTo.length, 'record read OK')

  const deleteResult = await alice.graphQL(`
    mutation($id: String!) {
      res: deleteProposal(id: $id)
    }
  `, {
    id: psId,
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
    id: psId,
  })

  t.equal(queryForDeleted.errors.length, 1, 'querying deleted record is an error')
  t.notEqual(-1, queryForDeleted.errors[0].message.indexOf('No entry at this address'), 'correct error reported')
})

runner.run()