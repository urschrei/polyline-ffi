#!/bin/bash

# This script takes care of testing your crate

set -ex

# TODO This is the "test phase", tweak it as you see fit
main() {

    if [ ! -z $DISABLE_TESTS ]; then
        return
    fi
    # don't test on aarch, since we can't install the linker
    if [ $TARGET = aarch64-unknown-linux-gnu ]; then
        return
    fi

    cross test --target $TARGET
}

# we don't run the "test phase" when doing deploys
if [ -z $TRAVIS_TAG ]; then
    main
fi
