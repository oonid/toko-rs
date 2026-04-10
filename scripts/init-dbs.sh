#!/bin/bash
set -e

psql -v ON_ERROR_STOP=0 --username "$POSTGRES_USER" <<-EOSQL
  SELECT 'CREATE DATABASE toko_test' WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'toko_test')\gexec
  SELECT 'CREATE DATABASE toko_e2e' WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'toko_e2e')\gexec
EOSQL
