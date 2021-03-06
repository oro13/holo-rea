/**
 * Commitment mutations
 *
 * @package: HoloREA
 * @since:   2019-08-28
 */

import { zomeFunction } from '../connection'
import {
  CommitmentCreateParams,
  CommitmentUpdateParams,
  CommitmentResponse,
} from '@valueflows/vf-graphql'

// :TODO: how to inject DNA identifier?
const createHandler = zomeFunction('planning', 'commitment', 'create_commitment')
const updateHandler = zomeFunction('planning', 'commitment', 'update_commitment')
const deleteHandler = zomeFunction('planning', 'commitment', 'delete_commitment')

// CREATE
interface CreateArgs {
  commitment: CommitmentCreateParams,
}
type createHandler = (root: any, args: CreateArgs) => Promise<CommitmentResponse>

export const createCommitment: createHandler = async (root, args) => {
  return (await createHandler)(args)
}

// UPDATE
interface UpdateArgs {
  commitment: CommitmentUpdateParams,
}
type updateHandler = (root: any, args: UpdateArgs) => Promise<CommitmentResponse>

export const updateCommitment: updateHandler = async (root, args) => {
  return (await updateHandler)(args)
}

// DELETE
type deleteHandler = (root: any, args: { id: string }) => Promise<boolean>

export const deleteCommitment: deleteHandler = async (root, args) => {
  return (await deleteHandler)({ address: args.id })
}
