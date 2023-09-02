#!/bin/bash

TAG=$(date -u -Iseconds | sed "s/:/_/g" | sed "s/\+/./g")

IMAGE_NAME=gitea.box/aaron/lernlotsen-rs-backend

set -e

docker buildx build . -t ${IMAGE_NAME}:latest

docker tag ${IMAGE_NAME}:latest ${IMAGE_NAME}:${TAG}

docker push ${IMAGE_NAME}:latest
docker push ${IMAGE_NAME}:${TAG}