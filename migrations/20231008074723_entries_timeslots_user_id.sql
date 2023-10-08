-- Add migration script here
ALTER TABLE timeslots
	ADD CONSTRAINT uniq_id_user_id UNIQUE (id, user_id);

ALTER TABLE entries
	ADD CONSTRAINT fk_id_user_id FOREIGN KEY (timeslot_id, user_id) REFERENCES timeslots(id, user_id);