#!/bin/bash

# shamelessly made with ChatGPT

# Database file path
DB_FILE="puzzle.db"

# Temporary file for SQL commands
SQL_FILE="modify_schema.sql"

# SQL commands to modify the schema
cat <<EOF > "$SQL_FILE"
-- Step 1: Create a temporary table
CREATE TABLE IF NOT EXISTS puzzles_temp (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    file TEXT NOT NULL UNIQUE
);

-- Step 2: Copy data from the original table to the temporary table
INSERT INTO puzzles_temp (id, name, file)
SELECT id, name, file FROM puzzles;

-- Step 3: Drop the original table
DROP TABLE IF EXISTS puzzles;

-- Step 4: Rename the temporary table to the original table's name
ALTER TABLE puzzles_temp RENAME TO puzzles;
EOF

# Execute SQL commands using SQLite CLI
sqlite3 "$DB_FILE" < "$SQL_FILE"

# Cleanup: remove the temporary SQL file
rm "$SQL_FILE"

echo "Schema modified successfully."
