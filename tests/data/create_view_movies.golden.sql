CREATE VIEW v_movie_details AS
SELECT
    m.id              AS movie_id,
    m.title           AS title,
    m.release_year    AS release_year,
    m.runtime_minutes AS runtime,
    d.name            AS director_name,
    CONCAT(m.title, ' (', m.release_year, ')') AS display_title
FROM movies m
    LEFT JOIN directors d
        ON d.id = m.director_id;
