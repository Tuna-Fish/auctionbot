#!/bin/bash
export DATABASE_HOST="address of database to connect to here"
export DATABASE_PW="database password here"
export DATABASE_USER="database user here"
export DISCORD_TOKEN="discord bot token here"
export RUST_LOG=auctionbot=info # logs to console, info | debug | none
./auctionbot
