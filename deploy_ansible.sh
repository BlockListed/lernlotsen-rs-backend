#!/bin/bash

cd ~/Documents/dev/ansible/Server

ansible-playbook deploy_lelo.yml --ask-vault-pass
