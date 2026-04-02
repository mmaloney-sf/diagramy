#!/bin/bash

set -ex

make web
rsync -ra web/* sifiveacademy1:www/diagramy/.
