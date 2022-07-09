.PHONY: toolchain-shell
build-shell: toolchain
	$(toolchain)

.PHONY: toolchain
toolchain:
	DOCKER_BUILDKIT=1 \
	docker build \
		--tag local/$(NAME)-build \
		--build-arg DEBIAN_HASH=$(DEBIAN_HASH) \
		.

toolchain := \
	docker run \
		--interactive \
		--rm \
		--user=$(shell id -u):$(shell id -g) \
		-v $(PWD)/$(CONFIG_DIR):/config \
		-v $(PWD)/$(KEY_DIR):/keys \
		-v $(PWD)/$(CACHE_DIR):/cache \
		-v $(PWD)/$(OUT_DIR):/out \
		-e GNUPGHOME=/cache/.gnupg \
		-t local/$(NAME)-build \
		bash -c
