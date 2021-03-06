/**
 * Helpers related to `key indexes`.
 *
 * A `key index` is a special form of `direct index`, where the `origin address` of the
 * link is a simple entry that contains only the address of some *other* entry.
 *
 * These are used to provide lookup behaviour where an entry needs to be referred to
 * and retrieved by a consistent ID which is not dependent upon the entry content,
 * as its native `Address` would be.
 *
 * :TODO: abstract remainder of the logic from unit_requests.rs into this module.
 *
 * @see     ../README.md
 * @package HDK Graph Helpers
 * @since   2019-05-16
 */
use hdk::{
    holochain_persistence_api::cas::content::Address,
    holochain_core_types::{
        entry::Entry::App as AppEntry,
        entry::entry_type::AppEntryType,
    },
    error::{ ZomeApiResult },
    entry_address,
    commit_entry,
    utils:: {
        get_as_type,    // :TODO: switch this method to one which doesn't consume the input
    },
};

//--------------------------------[ READ ]--------------------------------------

/// Query the `entry` address for a given `key index` address and return the result in an Address
/// NewType wrapper of the expected type.
///
pub fn get_key_index_address_as_type<A>(key_address: &Address) -> ZomeApiResult<A>
    where A: AsRef<Address> + From<Address>,
{
    let result: ZomeApiResult<Address> = get_as_type(key_address.clone());

    match result {
        Ok(res) => Ok(res.into()),
        Err(e) => Err(e),
    }
}

/// Query the underlying `entry` address for a given `key index` address and return as a raw Address
///
pub (crate) fn get_key_index_address(key_address: &Address) -> ZomeApiResult<Address> {
    get_as_type(key_address.clone())
}

/// Determine the underlying `entry` address for a given external `base_address`, without querying the DHT.
/// The `base_entry_type` must be provided in order to calculate the entry hash.
///
pub (crate) fn determine_key_index_address<A, S>(base_entry_type: S, base_address: &Address) -> ZomeApiResult<A>
    where S: Into<AppEntryType>,
        A: From<Address>,
{
    entry_address(&AppEntry(base_entry_type.into(), (*base_address).clone().into()))
        .map(|addr| { addr.into() })
}

//-------------------------------[ CREATE ]-------------------------------------

/// Creates a `key index`- an entry consisting only of a pointer to some other referenced
/// `entry`. The address of the `key index` entry (the alias the changing `entry` will be identified by
/// within this network) is returned.
pub (crate) fn create_key_index(
    base_entry_type: &AppEntryType,
    referenced_address: &Address,
) -> ZomeApiResult<Address> {
    let base_entry = AppEntry(base_entry_type.clone().into(), referenced_address.into());
    commit_entry(&base_entry)
}
