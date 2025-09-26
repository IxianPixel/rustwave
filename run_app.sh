#!/bin/bash

# Quick launcher script for Rustwave app bundle

echo "Building and launching Rustwave..."

# Build the app bundle
./build_app.sh

# Launch the app
echo "Launching app..."
open Rustwave.app

echo "Rustwave launched! Check your dock for the proper icon and name."
