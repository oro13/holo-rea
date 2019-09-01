/**
 * Handling for external request structure for economic event records
 */

use hdk::{
    holochain_persistence_api::{
        cas::content::Address,
    },
    holochain_json_api::{
        json::JsonString,
        error::JsonError,
    },
    holochain_core_types::link::LinkMatch::Exactly,
    error::ZomeApiResult,
    error::ZomeApiError,
    get_links,
};
use holochain_json_derive::{ DefaultJson };

use hdk_graph_helpers::{
    records::{
        create_record,
        read_record_entry,
        update_record,
        delete_record,
    },
    links::{
        get_links_and_load_entry_data,
    },
};
use vf_observation::economic_event::{
    Entry as EconomicEventEntry,
    CreateRequest as EconomicEventCreateRequest,
    UpdateRequest as EconomicEventUpdateRequest,
    ResponseData as EconomicEventResponse,
    construct_response,
};
use vf_observation::identifiers::{
    EVENT_BASE_ENTRY_TYPE,
    EVENT_ENTRY_TYPE,
    EVENT_INITIAL_ENTRY_LINK_TYPE,
    EVENT_FULFILLS_LINK_TYPE,
    EVENT_FULFILLS_LINK_TAG,
    EVENT_SATISFIES_LINK_TYPE,
    EVENT_SATISFIES_LINK_TAG,
};
use vf_planning::identifiers::{
    FULFILLMENT_FULFILLEDBY_LINK_TYPE,
    FULFILLMENT_FULFILLEDBY_LINK_TAG,
    SATISFACTION_SATISFIEDBY_LINK_TYPE,
    SATISFACTION_SATISFIEDBY_LINK_TAG,
};

#[derive(Serialize, Deserialize, Debug, DefaultJson, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QueryParams {
    satisfies: Option<Address>,
    fulfills: Option<Address>,
}

// API gateway entrypoints. All methods must accept parameters by value.

pub fn receive_create_economic_event(event: EconomicEventCreateRequest) -> ZomeApiResult<EconomicEventResponse> {
    handle_create_economic_event(&event)
}

pub fn receive_get_economic_event(address: Address) -> ZomeApiResult<EconomicEventResponse> {
    handle_get_economic_event(&address)
}

pub fn receive_update_economic_event(event: EconomicEventUpdateRequest) -> ZomeApiResult<EconomicEventResponse> {
    handle_update_economic_event(&event)
}

pub fn receive_delete_economic_event(address: Address) -> ZomeApiResult<bool> {
    delete_record::<EconomicEventEntry>(&address)
}

pub fn receive_query_events(params: QueryParams) -> ZomeApiResult<Vec<EconomicEventResponse>> {
    handle_query_events(&params)
}

// API logic handlers

fn handle_create_economic_event(event: &EconomicEventCreateRequest) -> ZomeApiResult<EconomicEventResponse> {
    let (base_address, entry_resp): (Address, EconomicEventEntry) = create_record(
        EVENT_BASE_ENTRY_TYPE, EVENT_ENTRY_TYPE,
        EVENT_INITIAL_ENTRY_LINK_TYPE,
        event.to_owned()
    )?;

    // return entire record structure
    Ok(construct_response(&base_address, &entry_resp, &None, &None))
}

fn handle_get_economic_event(address: &Address) -> ZomeApiResult<EconomicEventResponse> {
    let entry = read_record_entry(&address)?;

    Ok(construct_response(&address, &entry,
        &Some(get_fulfillment_ids(&address)?),
        &Some(get_satisfaction_ids(&address)?),
    ))
}

fn handle_update_economic_event(event: &EconomicEventUpdateRequest) -> ZomeApiResult<EconomicEventResponse> {
    let address = event.get_id();
    let new_entry = update_record(EVENT_ENTRY_TYPE, &address, event)?;

    Ok(construct_response(address, &new_entry,
        &Some(get_fulfillment_ids(&address)?),
        &Some(get_satisfaction_ids(&address)?),
    ))
}

fn handle_query_events(params: &QueryParams) -> ZomeApiResult<Vec<EconomicEventResponse>> {
    let mut entries_result: ZomeApiResult<Vec<(Address, Option<EconomicEventEntry>)>> = Err(ZomeApiError::Internal("No results found".to_string()));

    // :TODO: implement proper AND search rather than exclusive operations
    match &params.satisfies {
        Some(satisfies) => {
            entries_result = get_links_and_load_entry_data(
                &satisfies, SATISFACTION_SATISFIEDBY_LINK_TYPE, SATISFACTION_SATISFIEDBY_LINK_TAG,
            );
        },
        _ => (),
    };
    match &params.fulfills {
        Some(fulfills) => {
            entries_result = get_links_and_load_entry_data(
                &fulfills, FULFILLMENT_FULFILLEDBY_LINK_TYPE, FULFILLMENT_FULFILLEDBY_LINK_TAG,
            );
        },
        _ => (),
    };

    match entries_result {
        Ok(entries) => Ok(
            entries.iter()
                .map(|(entry_base_address, maybe_entry)| {
                    match maybe_entry {
                        Some(entry) => Ok(construct_response(
                            entry_base_address, &entry,
                            &Some(get_fulfillment_ids(&entry_base_address)?),
                            &Some(get_satisfaction_ids(&entry_base_address)?),
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

// Internals

/// Used to load the list of linked Fulfillment IDs
fn get_fulfillment_ids(economic_event: &Address) -> ZomeApiResult<Vec<Address>> {
    Ok(get_links(&economic_event, Exactly(EVENT_FULFILLS_LINK_TYPE), Exactly(EVENT_FULFILLS_LINK_TAG))?.addresses())
}

/// Used to load the list of linked Satisfaction IDs
fn get_satisfaction_ids(economic_event: &Address) -> ZomeApiResult<Vec<Address>> {
    Ok(get_links(&economic_event, Exactly(EVENT_SATISFIES_LINK_TYPE), Exactly(EVENT_SATISFIES_LINK_TAG))?.addresses())
}
