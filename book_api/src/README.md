sea-orm-cli generate entity -u postgres://postgres:postgres@localhost:5433/rust-superapp --with-serde both -o ./src/entity
DATABASE_URL=postgres://postgres:postgres@localhost:5433/rust-superapp cargo run

KAFKA creator
docker exec rust-superapp-broker \
                    kafka-topics --bootstrap-server broker:9092 \
                    --create \
                    --topic test-topic

docker exec -it rust-superapp-db bash
su postgres