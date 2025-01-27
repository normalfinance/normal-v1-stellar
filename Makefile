SUBDIRS := contracts/governor contracts/index_token_factory contracts/index_token contracts/insurance contracts/scheduler contracts/synth_pool contracts/synth_market contracts/synth_market_factory contracts/token contracts/vesting contracts/votes packages/normal packages/decimal packages/curve
BUILD_FLAGS ?=

default: build

all: test

build:
	@for dir in $(SUBDIRS) ; do \
		$(MAKE) -C $$dir build BUILD_FLAGS=$(BUILD_FLAGS) || exit 1; \
	done

test: build
	@for dir in $(SUBDIRS) ; do \
		$(MAKE) -C $$dir test BUILD_FLAGS=$(BUILD_FLAGS) || exit 1; \
	done

fmt:
	@for dir in $(SUBDIRS) ; do \
		$(MAKE) -C $$dir fmt || exit 1; \
	done

lints: fmt
	@for dir in contracts/multihop contracts/pool ; do \
		$(MAKE) -C $$dir build || exit 1; \
	done
	@for dir in $(SUBDIRS) ; do \
		$(MAKE) -C $$dir clippy || exit 1; \
	done

clean:
	@for dir in $(SUBDIRS) ; do \
		$(MAKE) -C $$dir clean || exit 1; \
	done
