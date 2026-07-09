CREATE VIEW v_images AS
SELECT
    id,
    folder,
    filename,
    s3_key,
    original_filename,
    title,
    description,
    tags,
    mime_type,
    width,
    height,
    file_size,
    created_at,
    updated_at,
    folder || '__' ||
    filename || '__' ||
    s3_key || '__' ||
    COALESCE(original_filename, '') || '__' ||
    COALESCE(title, '') || '__' ||
    COALESCE(description, '') || '__' ||
    COALESCE(tags, '') || '__' ||
    mime_type AS search_text
FROM images;
