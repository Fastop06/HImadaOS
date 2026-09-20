#!/bin/bash
echo "Waiting 12s for HimadaOS to boot, download Apache files over network, and start HTTP server..."
sleep 25
echo "Sending curl request to HimadaOS Apache Server (localhost:8081)..."
curl -v http://localhost:8081/ || true
