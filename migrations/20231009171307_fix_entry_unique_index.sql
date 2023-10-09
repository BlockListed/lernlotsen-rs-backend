-- Add migration script here
DROP INDEX entries_unique_index_index;

CREATE UNIQUE INDEX ON entries (index, timeslot_id);