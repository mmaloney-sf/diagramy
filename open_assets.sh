#!/bin/bash

make assets
for f in $(find assets/images -type f); do
    open $f
done
