#!/bin/bash
set -e

# Check if there is user deploy
if ! id "deploy" >/dev/null 2>&1; then
    # Create user "deploy"
    echo "creating user 'deploy' and group 'deploy'"
    groupadd deploy || echo 'group deploy already exists'
    useradd -m -s /bin/false -g deploy deploy
fi
