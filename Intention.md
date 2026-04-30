The intention is to build a personal finance tracker software with an API for interactions, we do not want to build a backend from scratch, rather use ledger-cli to handle the backend. We will need wrappers over ledger-cli. The reason here is less developer overhead, use a highly efficient system that is battle proven and also flatfiles for import and export.

My idea for development:

1. ledger-cli for the core software
2. File based transaction records
3. an API backend that wraps ledger
4. Front end can talk to backend via API
5. External tools can make transactions using API too
6. Rust the language of choice for the backend
7. Svelte is for frontend
8. I think we need a redis cache for quick reads and writes, while redis to file dump should be done using a worker periodically, any other suggestions welcome too
9. From a front end standpoint, we need individual user accounts and shared accounts. Each user will have one master file that further branches into single file per account. Users can invite other users into an account and then it becomes a shared account. Key here is to figure out how to bring user information in ledger, coz we may have to track how much an user spends even from shared accounts, without pulling information of other users in the same account
10. We aim to replicate the setup using docker-compose file, so the builds have to be a ready to spin docker image and compose combination. Choose a light weight container for the base image. Speed should be our standout feature.


Keep code simple, modular, readable. Follow industry standards. Optimise code for speed. This will be a FOSS code repo, so clean documentation and no junk comments in code please.  Let's get started and optimise further.