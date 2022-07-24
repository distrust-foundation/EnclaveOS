PWD := $(dir $(abspath $(firstword $(MAKEFILE_LIST))))

# Build toolchain container using pinned packages
$(OUT_DIR)/toolchain.tar:
	DOCKER_BUILDKIT=1 \
	docker build \
		--tag local/$(NAME)-build \
		--build-arg DEBIAN_HASH=$(DEBIAN_HASH) \
		--build-arg CONFIG_DIR=config \
		--build-arg SCRIPTS_DIR=scripts \
		$(PWD)
	docker save "local/$(NAME)-build" -o "$@"

define toolchain
	docker load -i $(OUT_DIR)/toolchain.tar
	docker run \
		--rm \
		--tty \
		--interactive \
		--user=$(1) \
		--platform=linux/$(ARCH) \
		--volume $(PWD)/$(CONFIG_DIR):/config \
		--volume $(PWD)/$(KEY_DIR):/keys \
		--volume $(PWD)/$(CACHE_DIR):/cache \
		--volume $(PWD)/$(OUT_DIR):/out \
		--volume $(PWD)/$(SCRIPTS_DIR):/scripts \
		--env GNUPGHOME=/cache/.gnupg \
		--env ARCH=$(ARCH) \
		--env KBUILD_BUILD_USER=$(KBUILD_BUILD_USER) \
		--env KBUILD_BUILD_HOST=$(KBUILD_BUILD_HOST) \
		--env KBUILD_BUILD_TIMESTAMP=$(KBUILD_BUILD_TIMESTAMP) \
		--env KCONFIG_NOTIMESTAMP=$(KCONFIG_NOTIMESTAMP) \
		--env SOURCE_DATE_EPOCH=$(SOURCE_DATE_EPOCH) \
		local/$(NAME)-build \
		bash -c $(2)
endef
