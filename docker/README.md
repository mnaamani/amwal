## Infrastrucure

## standup the containers
```sh
docker compose up -d
```

## check that the required databases have been created
```sh
docker exec amwal_db psql -U postgres -c '\l'
```