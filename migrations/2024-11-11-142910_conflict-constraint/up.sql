ALTER TABLE CONFLICT
    DROP CONSTRAINT conflict_creator_id_fkey;

ALTER TABLE CONFLICT
    ADD CONSTRAINT conflict_creator_id_fkey FOREIGN KEY (creator_id) REFERENCES person (id) ON UPDATE CASCADE ON DELETE CASCADE;

