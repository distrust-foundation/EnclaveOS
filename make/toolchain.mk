.PHONY: toolchain-shell
build-shell: toolchain
	$(toolchain) bash

.PHONY: toolchain
toolchain:
	DOCKER_BUILDKIT=1 \
	docker build \
		--tag local/$(NAME)-build \
		--build-arg DEBIAN_HASH=$(DEBIAN_HASH) \
		.

toolchain := \
	docker run \
		--rm \
		--interactive \
		--user=$(shell id -u):$(shell id -g) \
		-v $(PWD)/$(CONFIG_DIR):/config \
		-v $(PWD)/$(KEY_DIR):/keys \
		-v $(PWD)/$(CACHE_DIR):/cache \
		-v $(PWD)/$(OUT_DIR):/out \
		-v $(PWD)/$(SCRIPTS_DIR):/scripts \
		-e GNUPGHOME=/cache/.gnupg \
		-e KBUILD_BUILD_USER=$(KBUILD_BUILD_USER) \
		-e KBUILD_BUILD_HOST=$(KBUILD_BUILD_HOST) \
		-e KBUILD_BUILD_TIMESTAMP=$(KBUILD_BUILD_TIMESTAMP) \
		-e KCONFIG_NOTIMESTAMP=$(KCONFIG_NOTIMESTAMP) \
		-e SOURCE_DATE_EPOCH=$(SOURCE_DATE_EPOCH) \
		-t local/$(NAME)-build \
		bash -c
