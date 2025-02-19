SUBDIRS := contracts/token contracts/market contracts/market_factory contracts/insurance packages/decimal packages/normal 
BUILD_FLAGS ?=

default: build

all: test

build:
	@for dir in $(SUBDIRS) ; do \
		$(MAKE) -C $$dir build BUILD_FLAGS=$(BUILD_FLAGS) || exit 1; \
	done

# test: build
# 	@for dir in $(SUBDIRS) ; do \
# 		$(MAKE) -C $$dir test BUILD_FLAGS=$(BUILD_FLAGS) || exit 1; \
# 	done

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
