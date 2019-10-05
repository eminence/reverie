#!/bin/bash

TOPDIR=`realpath $(dirname $0)/..`

REVERIE=${TOPDIR}/target/release/reverie
PRELOADER=${TOPDIR}/target/release/libreverie_preloader.so
ECHO=${TOPDIR}/target/release/libcounter.so

unshare --mount-proc -Umpf ${REVERIE} --debug=${DEBUG} --preloader=${PRELOADER} --tool=${ECHO} --show-perf-stats -- $*
