/**
 * Holo-REA 'economic resource' zome library API
 *
 * Contains helper methods that can be used to manipulate economic resource data
 * structures in either the local Holochain zome, or a separate DNA-local zome.
 *
 * @package Holo-REA
 */
use std::borrow::Cow;
use hdk::{
    error::{ ZomeApiResult, ZomeApiError },
};

use hdk_graph_helpers::{
    records::{
        read_record_entry,
        update_record,
    },
    links::get_linked_addresses_as_type,
    anchors::read_anchored_record_entries,
    local_indexes::{
        replace_direct_index,
        query_direct_index_with_foreign_key,
        query_direct_remote_index_with_foreign_key,
    },
};

use hc_zome_rea_resource_specification_storage_consts::{
    ECONOMIC_RESOURCE_SPECIFICATION_BASE_ENTRY_TYPE,
    RESOURCE_SPECIFICATION_CONFORMING_RESOURCE_LINK_TYPE,
    RESOURCE_SPECIFICATION_CONFORMING_RESOURCE_LINK_TAG,
};

use vf_core::type_aliases::{
    ResourceAddress,
    EventAddress,
    ActionId,
    ProcessSpecificationAddress,
};

use hc_zome_rea_process_storage::Entry as ProcessEntry;
use hc_zome_rea_economic_resource_storage::*;
use hc_zome_rea_economic_resource_storage_consts::*;
use hc_zome_rea_economic_resource_rpc::*;
use hc_zome_rea_economic_event_storage::Entry as EventEntry;
use hc_zome_rea_economic_event_rpc::{
    CreateRequest as EventCreateRequest,
    ResourceResponse as Response,
    ResourceResponseData as ResponseData,
};

pub fn receive_get_economic_resource(address: ResourceAddress) -> ZomeApiResult<ResponseData> {
    handle_get_economic_resource(&address)
}

pub fn receive_update_economic_resource(resource: UpdateRequest) -> ZomeApiResult<ResponseData> {
    handle_update_economic_resource(&resource)
}

pub fn receive_get_all_economic_resources() -> ZomeApiResult<Vec<ResponseData>> {
    handle_get_all_economic_resources()
}

pub fn receive_query_economic_resources(params: QueryParams) -> ZomeApiResult<Vec<ResponseData>> {
    handle_query_economic_resources(&params)
}

fn handle_get_economic_resource(address: &ResourceAddress) -> ZomeApiResult<ResponseData> {
    let entry = read_record_entry(&address)?;
    Ok(construct_response(&address, &entry, get_link_fields(&address)))
}

fn handle_update_economic_resource(resource: &UpdateRequest) -> ZomeApiResult<ResponseData> {
    let address = resource.get_id();
    let new_entry = update_record(RESOURCE_ENTRY_TYPE, &address, resource)?;

    // :TODO: handle link fields
    replace_direct_index(address, &resource.get_contained_in(),
        RESOURCE_CONTAINED_IN_LINK_TYPE, RESOURCE_CONTAINED_IN_LINK_TAG,
        RESOURCE_CONTAINS_LINK_TYPE, RESOURCE_CONTAINS_LINK_TAG,
    )?;

    // :TODO: optimise this- should pass results from `replace_direct_index` instead of retrieving from `get_link_fields` where updates
    Ok(construct_response(address, &new_entry, get_link_fields(address)))
}

fn handle_get_all_economic_resources() -> ZomeApiResult<Vec<ResponseData>> {
    let entries_result: ZomeApiResult<Vec<(ResourceAddress, Option<Entry>)>> = read_anchored_record_entries(
        &RESOURCE_INDEX_ROOT_ENTRY_TYPE.to_string(), RESOURCE_INDEX_ENTRY_LINK_TYPE, &RESOURCE_INDEX_ROOT_ENTRY_ID.to_string(),
    );

    handle_list_output(entries_result)
}

fn handle_query_economic_resources(params: &QueryParams) -> ZomeApiResult<Vec<ResponseData>> {
    let mut entries_result: ZomeApiResult<Vec<(ResourceAddress, Option<Entry>)>> = Err(ZomeApiError::Internal("No results found".to_string()));

    match &params.contains {
        Some(contains) => {
            entries_result = query_direct_index_with_foreign_key(
                &contains, RESOURCE_CONTAINED_IN_LINK_TYPE, RESOURCE_CONTAINED_IN_LINK_TAG,
            );
        },
        _ => (),
    };
    match &params.contained_in {
        Some(contained_in) => {
            entries_result = query_direct_index_with_foreign_key(
                contained_in, RESOURCE_CONTAINS_LINK_TYPE, RESOURCE_CONTAINS_LINK_TAG,
            );
        },
        _ => (),
    };
    match &params.conforms_to {
        Some(conforms_to) => {
            entries_result = query_direct_remote_index_with_foreign_key(
                conforms_to, ECONOMIC_RESOURCE_SPECIFICATION_BASE_ENTRY_TYPE, RESOURCE_SPECIFICATION_CONFORMING_RESOURCE_LINK_TYPE, RESOURCE_SPECIFICATION_CONFORMING_RESOURCE_LINK_TAG,
            );
        },
        _ => (),
    };

    handle_list_output(entries_result)
}

fn handle_list_output(entries_result: ZomeApiResult<Vec<(ResourceAddress, Option<Entry>)>>) -> ZomeApiResult<Vec<ResponseData>> {
    match entries_result {
        Ok(entries) => Ok(
            entries.iter()
                .map(|(entry_base_address, maybe_entry)| {
                    match maybe_entry {
                        Some(entry) => Ok(construct_response(
                            entry_base_address, &entry, get_link_fields(entry_base_address)
                        )),
                        None => Err(ZomeApiError::Internal("referenced entry not found".to_string()))
                    }
                })
                .filter_map(Result::ok)
                .collect()
        ),
        _ => Err(ZomeApiError::Internal("could not load linked addresses".to_string()))
    }
}

pub fn resource_creation(event: &EventCreateRequest, resource: &CreateRequest) -> CreationPayload {
    CreationPayload {
        event: event.to_owned(),
        resource: resource.to_owned(),
    }
}

/// Create response from input DHT primitives
pub fn construct_response<'a>(
    address: &ResourceAddress, e: &Entry, (
        contained_in,
        stage,
        state,
        contains,
     ): (
        Option<ResourceAddress>,
        Option<ProcessSpecificationAddress>,
        Option<ActionId>,
        Option<Cow<'a, Vec<ResourceAddress>>>,
    ),
) -> ResponseData {
    ResponseData {
        economic_resource: construct_response_record(address, e, (contained_in, stage, state, contains))
    }
}

/// Create response from input DHT primitives
pub fn construct_response_record<'a>(
    address: &ResourceAddress, e: &Entry, (
        contained_in,
        stage,
        state,
        contains,
     ): (
        Option<ResourceAddress>,
        Option<ProcessSpecificationAddress>,
        Option<ActionId>,
        Option<Cow<'a, Vec<ResourceAddress>>>,
    ),
) -> Response {
    Response {
        // entry fields
        id: address.to_owned(),
        conforms_to: e.conforms_to.to_owned(),
        classified_as: e.classified_as.to_owned(),
        tracking_identifier: e.tracking_identifier.to_owned(),
        lot: e.lot.to_owned(),
        image: e.image.to_owned(),
        accounting_quantity: e.accounting_quantity.to_owned(),
        onhand_quantity: e.onhand_quantity.to_owned(),
        unit_of_effort: e.unit_of_effort.to_owned(),
        stage: stage.to_owned(),
        state: state.to_owned(),
        current_location: e.current_location.to_owned(),
        note: e.note.to_owned(),

        // link fields
        contained_in: contained_in.to_owned(),
        contains: contains.map(Cow::into_owned),
    }
}

// field list retrieval internals
// @see construct_response
pub fn get_link_fields<'a>(resource: &ResourceAddress) -> (
    Option<ResourceAddress>,
    Option<ProcessSpecificationAddress>,
    Option<ActionId>,
    Option<Cow<'a, Vec<ResourceAddress>>>,
) {
    (
        get_linked_addresses_as_type(resource, RESOURCE_CONTAINED_IN_LINK_TYPE, RESOURCE_CONTAINED_IN_LINK_TAG).into_owned().pop(),
        get_resource_stage(resource),
        get_resource_state(resource),
        Some(get_linked_addresses_as_type(resource, RESOURCE_CONTAINS_LINK_TYPE, RESOURCE_CONTAINS_LINK_TAG)),
    )
}

fn get_resource_state(resource: &ResourceAddress) -> Option<ActionId> {
    let events: Vec<EventAddress> = get_affecting_events(resource);

    // grab the most recent "pass" or "fail" action
    events.iter()
        .rev()
        .fold(None, move |result, event| {
            // already found it, just fall through
            // :TODO: figure out the Rust STL method to abort on first Some() value
            if let Some(_) = result {
                return result;
            }

            let entry: ZomeApiResult<EventEntry> = read_record_entry(event);
            match entry {
                Err(_) => result, // :TODO: this indicates some data integrity error
                Ok(entry) => {
                    match &*String::from(entry.action.clone()) {
                        "pass" | "fail" => Some(entry.action),  // found it! Return this as the current resource state.
                        _ => result,    // still not located, keep looking...
                    }
                },
            }
        })
}

fn get_resource_stage(resource: &ResourceAddress) -> Option<ProcessSpecificationAddress> {
    let events: Vec<EventAddress> = get_affecting_events(resource);

    // grab the most recent event with a process output association
    events.iter()
        .rev()
        .fold(None, move |result, event| {
            // already found it, just fall through
            // :TODO: figure out the Rust STL method to abort on first Some() value
            if let Some(_) = result {
                return result;
            }

            let entry: ZomeApiResult<EventEntry> = read_record_entry(event);
            match entry {
                Err(_) => result, // :TODO: this indicates some data integrity error
                Ok(entry) => {
                    match &entry.output_of {
                        Some(output_of) => {
                            // get the associated process
                            let maybe_process_entry: ZomeApiResult<ProcessEntry> = read_record_entry(output_of);
                            // check to see if it has an associated specification
                            match &maybe_process_entry {
                                Ok(process_entry) => match &process_entry.based_on {
                                    Some(based_on) => Some(based_on.to_owned()),   // found it!
                                    None => result, // still not located, keep looking...
                                },
                                Err(_) => result, // :TODO: this indicates some data integrity error
                            }
                        },
                        None => result,    // still not located, keep looking...
                    }
                },
            }
        })
}

/// Read all the EconomicEvents affecting a given EconomicResource
fn get_affecting_events(resource: &ResourceAddress) -> Vec<EventAddress> {
    get_linked_addresses_as_type(
        resource,
        RESOURCE_AFFECTED_BY_EVENT_LINK_TYPE,
        RESOURCE_AFFECTED_BY_EVENT_LINK_TAG,
    ).into_owned()
}
