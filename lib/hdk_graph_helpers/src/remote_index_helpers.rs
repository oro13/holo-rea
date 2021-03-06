/**
 * Helpers relating to `remote indexes`.
 *
 * A `remote index` is similar to a `local index`, except that it is composed of
 * two indexes which service queries on either side of the network boundary.
 *
 * On the `origin` side,
 *
 * @see     ../README.md
 * @package HDK Graph Helpers
 * @since   2019-05-16
 */
use std::fmt::Debug;
use hdk::{
    holochain_json_api::{ json::JsonString, error::JsonError },
    holochain_persistence_api::cas::content::Address,
    holochain_core_types::{
        entry::entry_type::AppEntryType,
    },
    error::{ ZomeApiResult, ZomeApiError },
};
use holochain_json_derive::{ DefaultJson };

use super::{
    MaybeUndefined,
    links::{
        get_linked_addresses_as_type,
    },
    keys::{
        create_key_index,
        get_key_index_address,
        determine_key_index_address,
    },
    local_indexes::{
        create_direct_index,
    },
    internals::{
        wipe_links_from_origin,
        dereferenced_link_matches,
        dereferenced_link_does_not_match,
    },
    rpc::{
        read_from_zome,
        build_zome_req_error,
    },
    identifiers::{ ERR_MSG_REMOTE_INDEXING_ERR },
};

// Common request format (zome trait) for linking remote entries in cooperating DNAs
#[derive(Serialize, Deserialize, Debug, DefaultJson, Clone)]
struct RemoteEntryLinkRequest {
    base_entry: Address,
    target_entries: Vec<Address>,
    removed_entries: Vec<Address>,
}

#[derive(Serialize, Deserialize, Debug, DefaultJson, Clone)]
pub struct RemoteEntryLinkResponse {
    indexes_created: Vec<ZomeApiResult<Address>>,
    indexes_removed: Vec<ZomeApiResult<()>>,
}

//-------------------------------[ CREATE ]-------------------------------------

/// Toplevel method for triggering a link creation flow between two records in
/// different DNAs. The calling DNA will have an `origin query index` created for
/// fetching the referenced remote IDs; the destination DNA will have a
/// `destination query index` created for querying the referenced records in full.
///
pub fn create_direct_remote_index(
    remote_dna_id: &str,
    remote_zome_id: &str,
    remote_zome_method: &str,
    remote_request_cap_token: Address,
    remote_base_entry_type: &str,
    origin_relationship_link_type: &str,
    origin_relationship_link_tag: &str,
    destination_relationship_link_type: &str,
    destination_relationship_link_tag: &str,
    source_base_address: &Address,
    target_base_addresses: Vec<Address>,
) -> Vec<ZomeApiResult<Address>> {
    let mut local_results = create_direct_remote_index_origin(
        remote_base_entry_type,
        origin_relationship_link_type,
        origin_relationship_link_tag,
        destination_relationship_link_type,
        destination_relationship_link_tag,
        source_base_address,
        target_base_addresses.clone(),
    );

    let mut remote_results = request_sync_direct_remote_index_destination(
        remote_dna_id,
        remote_zome_id,
        remote_zome_method,
        remote_request_cap_token,
        source_base_address,
        target_base_addresses,
        vec![],
    ).unwrap_or_else(|e| { vec![Err(e)] });

    local_results.append(&mut remote_results);
    local_results
}

/// Creates a 'origin' query index used for fetching and querying pointers to other
/// records that are stored externally to this DNA / zome.
///
/// In the local DNA, this consists of `key index` addresses for all referenced foreign
/// content, bidirectionally linked to the originating record for querying in either direction.
///
/// In the remote DNA, a corresponding remote query index is built via `create_direct_remote_index_destination`,
/// which is presumed to be linked to the other end of the specified `remote_zome_method`.
///
/// :TODO: return any errors encountered in internal link creation
///
fn create_direct_remote_index_origin(
    remote_base_entry_type: &str,
    origin_relationship_link_type: &str,
    origin_relationship_link_tag: &str,
    destination_relationship_link_type: &str,
    destination_relationship_link_tag: &str,
    source_base_address: &Address,
    target_base_addresses: Vec<Address>,
) -> Vec<ZomeApiResult<Address>> {
    // abort if target_base_addresses are empty
    if target_base_addresses.len() == 0 { return vec![] }

    // Build local index first (for reading linked record IDs from the `source_base_address`)
    let results: Vec<ZomeApiResult<Address>> = target_base_addresses.iter()
        .map(|base_entry_addr| {
            // create a base entry pointer for the referenced commitment
            let base_entry_result = create_key_index(&(remote_base_entry_type.to_string().into()), base_entry_addr);

            match &base_entry_result {
                Ok(base_address) => {
                    // link event to commitment by `fulfilled`/`fulfilledBy` edge
                    create_direct_index(
                        &source_base_address, base_address,
                        origin_relationship_link_type, origin_relationship_link_tag,
                        destination_relationship_link_type, destination_relationship_link_tag
                    );
                },
                _ => (),
            }

            base_entry_result
        })
        .collect();

    results
}

/// Ask another bridged DNA or zome to build a 'remote query index' to match the
/// one we have just created locally.
/// When calling zomes within the same DNA, use `hdk::THIS_INSTANCE` as `remote_dna_id`.
///
/// :TODO: implement bridge genesis callbacks & private chain entry to wire up cross-DNA link calls
/// :TODO: propagate errors from callee in error context, rather than masking them
/// :TODO: return indexes_removed to the caller
///
fn request_sync_direct_remote_index_destination(
    remote_dna_id: &str,
    remote_zome_id: &str,
    remote_zome_method: &str,
    remote_request_cap_token: Address,
    source_base_address: &Address,
    target_base_addresses: Vec<Address>,
    removed_base_addresses: Vec<Address>,
) -> ZomeApiResult<Vec<ZomeApiResult<Address>>> {
    // Call into remote DNA to enable target entries to setup data structures
    // for querying the associated remote entry records back out.
    let response: ZomeApiResult<RemoteEntryLinkResponse> = read_from_zome(
        remote_dna_id, remote_zome_id, remote_request_cap_token, remote_zome_method, RemoteEntryLinkRequest {
            base_entry: source_base_address.clone().into(),
            target_entries: target_base_addresses,
            removed_entries: removed_base_addresses,
        }.into()
    );

    match response {
        Ok(RemoteEntryLinkResponse { indexes_created, .. }) => {
            Ok(indexes_created) // :TODO: how to treat deletion errors?
        },
        Err(_) => Err(ZomeApiError::Internal(build_zome_req_error(ERR_MSG_REMOTE_INDEXING_ERR, remote_dna_id, remote_zome_id, remote_zome_method))),
    }
}

/// Respond to a request from an external source to build a link index for some externally linking content.
///
/// This essentially creates a base link for the `source_base_address` and then links it to every
/// `target_base_addresses` found locally within this DNA.
///
/// The returned `RemoteEntryLinkResponse` provides an appropriate format for responding to indexing
/// requests that originate with a call to `create_remote_index_pair` in a foreign DNA.
///
pub fn handle_sync_direct_remote_index_destination<'a, A, B>(
    remote_base_entry_type: &'a str,
    origin_relationship_link_type: &str,
    origin_relationship_link_tag: &str,
    destination_relationship_link_type: &str,
    destination_relationship_link_tag: &str,
    source_base_address: &A,
    target_base_addresses: Vec<B>,
    removed_base_addresses: Vec<B>,
) -> ZomeApiResult<RemoteEntryLinkResponse>
    where A: AsRef<Address> + From<Address> + Clone,
        B: AsRef<Address> + From<Address> + Clone + PartialEq + Debug,
        Address: From<B>,
{
    // remove passed stale indexes
    let remove_resp = delete_direct_remote_index_destination(
        source_base_address,
        removed_base_addresses,
        remote_base_entry_type,
        origin_relationship_link_type, origin_relationship_link_tag,
        destination_relationship_link_type, destination_relationship_link_tag,
    );

    // create any new indexes
    let create_resp = create_direct_remote_index_destination(
        remote_base_entry_type,
        origin_relationship_link_type, origin_relationship_link_tag,
        destination_relationship_link_type, destination_relationship_link_tag,
        source_base_address,
        target_base_addresses,
    );
    if let Err(index_creation_failure) = create_resp {
        return Err(index_creation_failure);
    }

    Ok(RemoteEntryLinkResponse { indexes_created: create_resp.unwrap(), indexes_removed: remove_resp })
}

/// Creates a 'destination' query index used for following a link from some external record
/// into records contained within the current DNA / zome.
///
/// This basically consists of a `key index` for the remote content and bidirectional
/// links between it and its `target_base_addresses`.
///
/// :TODO: return any errors encountered in internal link creation
///
pub fn create_direct_remote_index_destination<'a, A, B>(
    remote_base_entry_type: &'a str,
    origin_relationship_link_type: &str,
    origin_relationship_link_tag: &str,
    destination_relationship_link_type: &str,
    destination_relationship_link_tag: &str,
    source_base_address: &A,
    target_base_addresses: Vec<B>,
) -> ZomeApiResult<Vec<ZomeApiResult<Address>>>
    where A: AsRef<Address> + From<Address> + Clone,
        B: AsRef<Address> + From<Address> + Clone + PartialEq,
{
    // create a base entry pointer for the referenced origin record
    let base_entry: AppEntryType = remote_base_entry_type.to_string().into();
    let base_resp = create_key_index(&base_entry, source_base_address.as_ref());
    if let Err(base_creation_failure) = base_resp {
        return Err(base_creation_failure);
    }
    let base_address = base_resp.unwrap();

    // link all referenced records to our pointer to the remote origin record
    Ok(target_base_addresses.iter()
        .map(|target_address| {
            // link origin record to local records by specified edge
            create_direct_index(
                &base_address, target_address.as_ref(),
                origin_relationship_link_type, origin_relationship_link_tag,
                destination_relationship_link_type, destination_relationship_link_tag
            );

            Ok(target_address.as_ref().clone())
        })
        .collect()
    )
}

//-------------------------------[ UPDATE ]-------------------------------------

/// Toplevel method for triggering a link update flow between two records in
/// different DNAs. Indexes on both sides of the network boundary will be updated.
///
/// :TODO: update to accept multiple targets for the replacement links
///
pub fn update_direct_remote_index<A, B, S>(
    remote_dna_id: &str,
    remote_zome_id: &str,
    remote_zome_method: &str,
    remote_request_cap_token: Address,
    remote_base_entry_type: S,
    origin_relationship_link_type: &str,
    origin_relationship_link_tag: &str,
    destination_relationship_link_type: &str,
    destination_relationship_link_tag: &str,
    source_base_address: &A,
    target_base_address: &MaybeUndefined<B>,
) -> ZomeApiResult<Vec<ZomeApiResult<Address>>>
    where A: AsRef<Address> + From<Address> + Clone,
        B: AsRef<Address> + From<Address> + Clone + PartialEq + Debug,
        S: Into<AppEntryType>,
{
    // no change, bail early
    if let MaybeUndefined::Undefined = target_base_address {
        return Ok(vec![]);
    }

    // process local index first and collect all removed link target address
    // :TODO: error handling
    let removed_links = replace_direct_remote_index_origin(
        source_base_address,
        target_base_address,
        remote_base_entry_type,
        origin_relationship_link_type,
        origin_relationship_link_tag,
        destination_relationship_link_type,
        destination_relationship_link_tag,
    )?.iter()
        .filter_map(|r| { r.clone().ok() })
        .collect();

    // pass removed IDs and new IDs to remote DNA for re-indexing
    request_sync_direct_remote_index_destination(
        remote_dna_id,
        remote_zome_id,
        remote_zome_method,
        remote_request_cap_token,
        source_base_address.as_ref(),
        match &target_base_address {
            &MaybeUndefined::Some(target) => vec![target.as_ref().clone()],
            _ => vec![],
        },
        removed_links,
    )
}

/// Same as `replace_direct_index` except that the replaced links
/// are matched against dereferenced addresses pointing to entries in other DNAs.
///
/// Returns the addresses of the previously erased link targets, if any.
///
/// :TODO: update to accept multiple targets for the replacement links
///
pub fn replace_direct_remote_index_origin<A, B, S>(
    source: &A,
    new_dest: &MaybeUndefined<B>,
    base_entry_type: S,
    link_type: &str,
    link_name: &str,
    link_type_reciprocal: &str,
    link_name_reciprocal: &str,
) -> ZomeApiResult<Vec<ZomeApiResult<Address>>>
    where A: AsRef<Address> + From<Address> + Clone,
        B: AsRef<Address> + From<Address> + Clone + PartialEq + Debug,
        S: Into<AppEntryType>,
{
    // if not updating, skip operation
    if let MaybeUndefined::Undefined = new_dest {
        return Ok(vec![]);
    }

    // load any existing links from the originating address
    let existing_links: Vec<B> = get_linked_addresses_as_type(source, link_type, link_name).into_owned();

    // determine links to erase
    let to_erase: Vec<B> = existing_links.iter()
        .filter(dereferenced_link_does_not_match(new_dest)).map(|x| { (*x).clone() }).collect();

    // wipe stale links. Note we don't remove the base addresses, dangling remnants do no harm.
    // :TODO: propagate errors
    let _erased: Vec<ZomeApiResult<()>> = to_erase.iter().flat_map(wipe_links_from_origin(
        link_type, link_name,
        link_type_reciprocal, link_name_reciprocal,
        source,
    )).collect();

    // get base addresses of erased items
    let erased: Vec<ZomeApiResult<Address>> = to_erase.iter().map(|addr| { get_key_index_address(addr.as_ref()) }).collect();

    // run insert if needed
    match new_dest {
        MaybeUndefined::Some(new_link) => {
            let already_present = existing_links.iter().filter(dereferenced_link_matches(new_dest)).count() > 0;

            if already_present {
                Ok(erased)
            } else {
                let new_dest_pointer = create_key_index(&(base_entry_type.into()), new_link.as_ref());
                if let Err(e) = new_dest_pointer {
                    return Err(e);
                }
                create_direct_index(
                    source.as_ref(), &(new_dest_pointer.unwrap()),
                    link_type, link_name,
                    link_type_reciprocal, link_name_reciprocal
                );  // :TODO: error handling
                Ok(erased)
            }
        },
        _ => Ok(erased),
    }
}

//-------------------------------[ DELETE ]-------------------------------------

/// Toplevel method for triggering a link update flow between two records in
/// different DNAs. Indexes on both sides of the network boundary will be updated.
///
/// :TODO: update to accept multiple targets for the replacement links
///
pub fn remove_direct_remote_index<'a, A, B>(
    remote_dna_id: &str,
    remote_zome_id: &str,
    remote_zome_method: &str,
    remote_request_cap_token: Address,
    remote_base_entry_type: &'a str,
    origin_relationship_link_type: &str,
    origin_relationship_link_tag: &str,
    destination_relationship_link_type: &str,
    destination_relationship_link_tag: &str,
    source_base_address: &A,
    remove_base_address: &B,
) -> Vec<ZomeApiResult<()>>
    where A: AsRef<Address> + From<Address> + Clone,
        B: AsRef<Address> + From<Address> + Clone + PartialEq + Debug,
        Address: From<B>,
{
    // process local index first and collect any errors
    let mut local_results = delete_direct_remote_index_destination(
        source_base_address,
        vec![remove_base_address.clone().into()],
        remote_base_entry_type,
        origin_relationship_link_type,
        origin_relationship_link_tag,
        destination_relationship_link_type,
        destination_relationship_link_tag,
    );

    // pass removed IDs and new IDs to remote DNA for re-indexing
    let remote_results = request_sync_direct_remote_index_destination(
        remote_dna_id,
        remote_zome_id,
        remote_zome_method,
        remote_request_cap_token,
        source_base_address.as_ref(),
        vec![],
        vec![remove_base_address.as_ref().clone()],
    );

    match remote_results {
        Ok(results) => {
            let mut remote_errors: Vec<ZomeApiResult<()>> = results.iter()
                .filter(|r| { r.is_err() })
                .map(|e| { Err(e.clone().err().unwrap()) })
                .collect();

            local_results.append(&mut remote_errors);
        },
        Err(e) => local_results.push(Err(e)),
    }

    local_results
}

/// Deletes a set of links between a remote record reference and some set
/// of local target addresses.
///
/// The 'base' entry representing the remote target is not
/// affected in the removal, and is simply left dangling in the
/// DHT space as an indicator of previously linked items.
///
pub (crate) fn delete_direct_remote_index_destination<'a, A, B>(
    source: &A,
    remove_targets: Vec<B>,
    base_entry_type: &'a str,
    link_type: &str,
    link_name: &str,
    link_type_reciprocal: &str,
    link_name_reciprocal: &str,
) -> Vec<ZomeApiResult<()>>
    where A: AsRef<Address> + From<Address> + Clone,
        B: AsRef<Address> + From<Address> + Clone + PartialEq + Debug,
        Address: From<B>,
{
    let dereferenced_source: ZomeApiResult<A> = determine_key_index_address(base_entry_type.to_string(), source.as_ref());
    if let Err(e) = dereferenced_source {
        return vec![Err(e)]
    }

    let index_address = dereferenced_source.unwrap();
    remove_targets.iter()
        .flat_map(wipe_links_from_origin(
            link_type, link_name,
            link_type_reciprocal, link_name_reciprocal,
            &index_address,
        ))
        .collect()
}
