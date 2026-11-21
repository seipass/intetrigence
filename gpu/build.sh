#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

mkdir -p build
glslc -O --target-env=vulkan1.2 -I shaders shaders/expand.comp -o build/expand.spv
glslc -O --target-env=vulkan1.2 -I shaders shaders/select.comp -o build/select.spv
spirv-val build/expand.spv
spirv-val build/select.spv
echo "shaders compiled and validated: OK"

if [ -f host/intetrigence_gpu.c ]; then
  INC_FLAGS=""
  if [ -d /tmp/vulkan-headers/usr/include ]; then
    INC_FLAGS="-I/tmp/vulkan-headers/usr/include"
  fi
  gcc -O3 -Wall -Wextra $INC_FLAGS host/intetrigence_gpu.c -o build/intetrigence_gpu -lvulkan -lm
  echo "built: gpu/build/intetrigence_gpu"
fi
