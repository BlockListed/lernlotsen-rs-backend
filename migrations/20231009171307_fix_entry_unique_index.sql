-- Add migration script here
DROP INDEX entries_unique_index_index;

CREATE UNIQUE INDEX entries_index_timeslot_id_idx ON entries (index, timeslot_id);