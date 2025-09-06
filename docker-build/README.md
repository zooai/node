## Local build

Inside the folder `docker-build` run:

```sh
DOCKER_BUILDKIT=1 docker build -f Dockerfile-RELEASE -t dcspark/zoo-node:latest ../
```

Inside the folder `docker-build`, to start the services, run:

```sh
INITIAL_AGENT_API_KEYS=sk-abc,sk-abc docker compose up -d
```

The following configuration items can be set from environment:
- __INITIAL_AGENT_NAMES__=${INITIAL_AGENT_NAMES:-openai_gpt,openai_gpt_vision}
- __INITIAL_AGENT_MODELS__=${INITIAL_AGENT_MODELS:-openai:gpt-4-1106-preview,openai:gpt-4-vision-preview}
- __INITIAL_AGENT_URLS__=${INITIAL_AGENT_URLS:-https://api.openai.com,https://api.openai.com}
- __INITIAL_AGENT_API_KEYS__=${INITIAL_AGENT_API_KEYS}

Point Visor to `http://127.0.0.1:9550`

## Prepare for partner

Inside the folder `docker-build` run:

```sh
sh partner_prepare.sh
```

output example:

```sh
$ sh partner_prepare.sh

[INFO]  Docker building dcspark/zoo-node:latest using Dockerfile-RELEASE with source at ../
sha256:b5fe5c4c8fc6229c15ea0cbde4881c090a0dcd72a1f6f8f42d29d7f9bfc8b4be
[INFO]  Preparing docker compose environment at zoo_deploy
[INFO]  Docker save dcspark/zoo-node:latest to dcspark_zoo-node.tar
[INFO]  Preparing partner data at zoo_deploy_partner/zoo_deploy.tar.gz
[INFO]  Cleaning zoo_deploy
[INFO]  Send to partner the file zoo_deploy_partner/zoo_deploy.tar.gz
```

Send to partner the final output generated at `zoo_deploy_partner/zoo_deploy.tar.gz`

## Partner info

Partner extracts the file `tar xzvf zoo_deploy.tar.gz`

```sh
$ tar xzvf zoo_deploy.tar.gz

zoo_deploy/
zoo_deploy/.env
zoo_deploy/docker-compose.yml
zoo_deploy/prepare.sh
zoo_deploy/dcspark_zoo-node.tar
```

and ends up with a folder `zoo_deploy` containing:

```sh
zoo_deploy
├── dcspark_zoo-node.tar
├── docker-compose.yml
├── .env
└── prepare.sh
```

runs `sh prepare.sh` that outputs additional information:

```sh
$ sh prepare.sh

[INFO]  Docker loading dcspark_zoo-node.tar
Loaded image: dcspark/zoo-node:latest
[INFO]  Edit ".env" if you want to start the node with preconfigured ai agents. You have the possibility to add ai agents also from Zoo Visor.
[INFO]  Once done with ".env" changes, to start on-prem infrastructure run: docker compose up -d
[INFO]  Once everything is up and running, install/start Zoo Visor and use the default provided settings on the ui.
```

final step is to run `docker compose up -d`.