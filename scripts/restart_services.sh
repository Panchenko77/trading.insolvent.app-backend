#!/bin/bash
systemctl --user daemon-reload
systemctl --user restart trading_auth
systemctl --user restart trading_user


