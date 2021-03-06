/**
 * Holo-REA proposal zome internal data structures
 *
 * Required by the zome itself, and for any DNA-local zomes interacting with its
 * storage API directly.
 *
 * @package Holo-REA
 */
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use holochain_json_api::{error::JsonError, json::JsonString};
use holochain_json_derive::DefaultJson;

use hdk_graph_helpers::{record_interface::Updateable, MaybeUndefined};

use vf_core::type_aliases::Timestamp;

use hc_zome_rea_proposal_rpc::{CreateRequest, UpdateRequest};

//---------------- RECORD INTERNALS & VALIDATION ----------------

#[derive(Serialize, Deserialize, Debug, DefaultJson, Clone)]
pub struct Entry {
    pub name: Option<String>,
    pub has_beginning: Option<Timestamp>,
    pub has_end: Option<Timestamp>,
    pub unit_based: Option<bool>,
    pub created: Option<Timestamp>,
    pub note: Option<String>,
    pub in_scope_of: Option<Vec<String>>,
    //[TODO]:
    //eligibleLocation: SpatialThing
    //publishes: [ProposedIntent!]
}

//---------------- CREATE ----------------

/// Pick relevant fields out of I/O record into underlying DHT entry
impl From<CreateRequest> for Entry {
    fn from(e: CreateRequest) -> Entry {
        Entry {
            name: e.name.into(),
            has_beginning: e.has_beginning.into(),
            has_end: e.has_end.into(),
            unit_based: e.unit_based.into(),
            created: e.created.into(),
            note: e.note.into(),
            in_scope_of: e.in_scope_of.to_option(),
        }
    }
}

//---------------- UPDATE ----------------

/// Handles update operations by merging any newly provided fields
impl Updateable<UpdateRequest> for Entry {
    fn update_with(&self, e: &UpdateRequest) -> Entry {
        Entry {
            name: if !e.name.is_some() {
                self.name.to_owned()
            } else {
                e.name.to_owned().into()
            },
            has_beginning: if e.has_beginning == MaybeUndefined::Undefined {
                self.has_beginning.to_owned()
            } else {
                e.has_beginning.to_owned().into()
            },
            has_end: if e.has_end == MaybeUndefined::Undefined {
                self.has_end.to_owned()
            } else {
                e.has_end.to_owned().into()
            },
            unit_based: if e.unit_based == MaybeUndefined::Undefined {
                self.unit_based.to_owned()
            } else {
                e.unit_based.to_owned().into()
            },
            created: self.created.to_owned(),
            note: if e.note == MaybeUndefined::Undefined {
                self.note.to_owned()
            } else {
                e.note.to_owned().into()
            },
            in_scope_of: if e.in_scope_of == MaybeUndefined::Undefined {
                self.in_scope_of.to_owned()
            } else {
                e.in_scope_of.to_owned().to_option()
            },
        }
    }
}
