#!/bin/sh
set -e
set -o noglob


SED_COMMAND=sed
COPY_COMMAND=cp

ZOO_NODE_IMAGE=${ZOO_NODE_IMAGE:-dcspark/zoo-node}
ZOO_NODE_VERSION=${ZOO_NODE_VERSION:-latest}

ZOO_COMPOSE_FILE=docker-compose.yml
ZOO_NODE_DOCKERFILE=Dockerfile-RELEASE

ZOO_NODE_ARCHIVE=dcspark_zoo-node.tar
ZOO_SOURCE_PATH=../

DOCKER_BUILD_CMD="docker build --quiet"
DOCKER_COMPOSE_CMD="docker compose" # docker-compose
DOCKER_LOAD_CMD="docker load --input"
DOCKER_SAVE_CMD="docker save --output"

ZOO_TMP_LOCAL_FOLDER=zoo_deploy
ZOO_TMP_PARTNER_FOLDER=zoo_deploy_partner
DOCKER_COMPOSE_ENV_FILE=.env
DOCKER_COMPOSE_ENV_DATA=$(cat << EOF
#
# single agent example
#
#INITIAL_AGENT_NAMES=openai_gpt
#INITIAL_AGENT_URLS=https://api.openai.com
#INITIAL_AGENT_MODELS=openai:gpt-4-1106-preview
#INITIAL_AGENT_API_KEYS=sk-abc
#
# multi agent example
#
#INITIAL_AGENT_NAMES=openai_gpt,openai_gpt_vision
#INITIAL_AGENT_URLS=https://api.openai.com,https://api.openai.com
#INITIAL_AGENT_MODELS=openai:gpt-4-1106-preview,openai:gpt-4-vision-preview
#INITIAL_AGENT_API_KEYS=sk-abc,sk-abc
#
# default none
#
INITIAL_AGENT_NAMES=
INITIAL_AGENT_URLS=
INITIAL_AGENT_MODELS=
INITIAL_AGENT_API_KEYS=
EOF
)



PARTNER_PREPARE_SCRIPT=$(cat << EOF
#!/bin/sh
set -e
set -o noglob

ZOO_NODE_ARCHIVE=dcspark_zoo-node.tar
DOCKER_LOAD_CMD="docker load --input"
DOCKER_COMPOSE_CMD="docker compose" # docker-compose
DOCKER_COMPOSE_ENV_FILE=.env


# --- helper functions for logs ---
info() {
  echo '[INFO] ' "\$@"
}
warn() {
  echo '[WARN] ' "\$@" >&2
}
fatal() {
  echo '[ERRO] ' "\$@" >&2
  exit 1
}

# --- load image ---
load_docker_image() {
  msg="Docker loading \${ZOO_NODE_ARCHIVE}"
  if [ -f \${ZOO_NODE_ARCHIVE} ]; then
    info \${msg}
    \${DOCKER_LOAD_CMD} \${ZOO_NODE_ARCHIVE}
  else
    fatal "\${msg} - failed (missing file - \${ZOO_NODE_ARCHIVE})"
  fi
}

# --- info about initial agents configuration ---
post_prepare_env_info() {
  msg="Edit \"\${DOCKER_COMPOSE_ENV_FILE}\" if you want to start the node with preconfigured ai agents. You have the possibility to add ai agents also from Zoo Visor."
  info \${msg}
}

# --- info docker compose ---
post_prepare_compose_info() {
  msg="Once done with \"\${DOCKER_COMPOSE_ENV_FILE}\" changes, to start on-prem infrastructure run: \${DOCKER_COMPOSE_CMD} up -d"
  info \${msg}
}


# --- info visor ---
post_prepare_visor_info() {
  msg="Once everything is up and running, install/start Zoo Visor and use the default provided settings on the ui."
  info \${msg}
}

load_docker_image
post_prepare_env_info
post_prepare_compose_info
post_prepare_visor_info

EOF
)

# --- helper functions for logs ---
info() {
  echo '[INFO] ' "$@"
}
warn() {
  echo '[WARN] ' "$@" >&2
}
fatal() {
  echo '[ERRO] ' "$@" >&2
  exit 1
}

# write $1 (content) to $2 (file)
write_to_file() {
  echo "$1" >| "$2" || fatal "failed to write data to $2"
}

# --- build image ---
build_docker_image() {
  msg="Docker building ${ZOO_NODE_IMAGE}:${ZOO_NODE_VERSION} using ${ZOO_NODE_DOCKERFILE} with source at ${ZOO_SOURCE_PATH}"
  if [ -f ${ZOO_NODE_DOCKERFILE} ]; then
    info ${msg}
    export DOCKER_BUILDKIT=1
    ${DOCKER_BUILD_CMD} -f ${ZOO_NODE_DOCKERFILE} -t ${ZOO_NODE_IMAGE}:${ZOO_NODE_VERSION} ${ZOO_SOURCE_PATH}
  else
    fatal "${msg} - failed (missing file - ${ZOO_NODE_DOCKERFILE})"
  fi
}

# --- save image ---
save_docker_image() {
  if [ ! -d "${ZOO_TMP_LOCAL_FOLDER}" ]; then
    mkdir ${ZOO_TMP_LOCAL_FOLDER} || fatal "failed to create local folder ${ZOO_TMP_LOCAL_FOLDER}"
  fi
  msg="Docker save ${ZOO_NODE_IMAGE}:${ZOO_NODE_VERSION} to ${ZOO_NODE_ARCHIVE}"
  if [ ! -f ${ZOO_NODE_ARCHIVE} ]; then
    info ${msg}
    ${DOCKER_SAVE_CMD} ${ZOO_TMP_LOCAL_FOLDER}/${ZOO_NODE_ARCHIVE} ${ZOO_NODE_IMAGE}:${ZOO_NODE_VERSION}
  else
    fatal "${msg} - failed (file already exists - ${ZOO_TMP_LOCAL_FOLDER}/${ZOO_NODE_ARCHIVE})"
  fi
}

# --- prepare docker-compose infra for partner ---
prepare_docker_compose() {
  msg="Preparing docker compose environment at ${ZOO_TMP_LOCAL_FOLDER}"
  if [ ! -d "${ZOO_TMP_LOCAL_FOLDER}" ]; then
    mkdir ${ZOO_TMP_LOCAL_FOLDER} || fatal "failed to create local folder ${ZOO_TMP_LOCAL_FOLDER}"
  fi
  info ${msg}
  # copy original compose file
  ${COPY_COMMAND} ${ZOO_COMPOSE_FILE} ${ZOO_TMP_LOCAL_FOLDER}/${ZOO_COMPOSE_FILE} || fatal "failed to copy ${ZOO_TMP_LOCAL_FOLDER}/${ZOO_COMPOSE_FILE}"
  # replace any INITIAL_AGENT_* initial value with envs
  ${SED_COMMAND} -i "s/INITIAL_AGENT_NAMES=.*/INITIAL_AGENT_NAMES=\${INITIAL_AGENT_NAMES}/g" ${ZOO_TMP_LOCAL_FOLDER}/${ZOO_COMPOSE_FILE} || fatal 'sed failed - INITIAL_AGENT_NAMES'
  ${SED_COMMAND} -i "s/INITIAL_AGENT_URLS=.*/INITIAL_AGENT_URLS=\${INITIAL_AGENT_URLS}/g" ${ZOO_TMP_LOCAL_FOLDER}/${ZOO_COMPOSE_FILE} || fatal 'sed failed - INITIAL_AGENT_URLS'
  ${SED_COMMAND} -i "s/INITIAL_AGENT_MODELS=.*/INITIAL_AGENT_MODELS=\${INITIAL_AGENT_MODELS}/g" ${ZOO_TMP_LOCAL_FOLDER}/${ZOO_COMPOSE_FILE} || fatal 'sed failed - INITIAL_AGENT_MODELS'
  ${SED_COMMAND} -i "s/INITIAL_AGENT_API_KEYS=.*/INITIAL_AGENT_API_KEYS=\${INITIAL_AGENT_API_KEYS}/g" ${ZOO_TMP_LOCAL_FOLDER}/${ZOO_COMPOSE_FILE} || fatal 'sed failed - INITIAL_AGENT_API_KEYS'
  # write .env sample file
  write_to_file "${DOCKER_COMPOSE_ENV_DATA}" ${ZOO_TMP_LOCAL_FOLDER}/${DOCKER_COMPOSE_ENV_FILE}
  # write partner prepare.sh
  write_to_file "${PARTNER_PREPARE_SCRIPT}" ${ZOO_TMP_LOCAL_FOLDER}/prepare.sh

}

# --- load image ---
load_docker_image() {
  msg="Docker loading ${ZOO_NODE_ARCHIVE}"
  if [ -f ${ZOO_NODE_ARCHIVE} ]; then
    info ${msg}
    ${DOCKER_LOAD_CMD} ${ZOO_NODE_ARCHIVE}
  else
    fatal "${msg} - failed (missing file - ${ZOO_NODE_ARCHIVE})"
  fi
}

# --- prepare partner archive ---
prepare_partner_archive() {
  msg="Preparing partner data at ${ZOO_TMP_PARTNER_FOLDER}/${ZOO_TMP_LOCAL_FOLDER}.tar.gz"
  if [ ! -d "${ZOO_TMP_PARTNER_FOLDER}" ]; then
    mkdir ${ZOO_TMP_PARTNER_FOLDER} || fatal "failed to create local folder ${ZOO_TMP_PARTNER_FOLDER}"
  fi
  info ${msg}
  tar -zcf ${ZOO_TMP_PARTNER_FOLDER}/${ZOO_TMP_LOCAL_FOLDER}.tar.gz ${ZOO_TMP_LOCAL_FOLDER}
}


# --- clean temp partner folder ---
clean_temporary_folder() {
  msg="Cleaning ${ZOO_TMP_LOCAL_FOLDER}"
  if [ -d "${ZOO_TMP_LOCAL_FOLDER}" ]; then
    info ${msg}
    rm -rf ${ZOO_TMP_LOCAL_FOLDER} || fatal "failed delete local folder ${ZOO_TMP_LOCAL_FOLDER}"
  fi
}

# --- info what to send to partner  ---
partner_file_info() {
  msg="Send to partner the file ${ZOO_TMP_PARTNER_FOLDER}/${ZOO_TMP_LOCAL_FOLDER}.tar.gz"
  if [ -f "${ZOO_TMP_PARTNER_FOLDER}/${ZOO_TMP_LOCAL_FOLDER}.tar.gz" ]; then
    info ${msg}
  else
    fatal "${msg} - error (missing file - ${ZOO_TMP_PARTNER_FOLDER}/${ZOO_TMP_LOCAL_FOLDER}.tar.gz)"
  fi
}


build_docker_image
prepare_docker_compose
save_docker_image
prepare_partner_archive
clean_temporary_folder
partner_file_info