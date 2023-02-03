TARGET := generic
include $(PWD)/src/toolchain/Makefile

ifeq ($(TARGET), aws)
DEFAULT_GOAL := $(OUT_DIR)/$(ARCH).eif
else ifeq ($(TARGET), generic)
DEFAULT_GOAL := $(OUT_DIR)/$(ARCH).bzImage
endif
.DEFAULT_GOAL :=
default: toolchain $(DEFAULT_GOAL)

# Clean repo back to initial clone state
.PHONY: clean
clean:
	rm -rf cache out
	docker image rm -f local/$(NAME)-build

# Build latest image and run in terminal via Qemu
.PHONY: run
run: $(OUT_DIR)/bzImage
	qemu-system-x86_64 \
		-m 512M \
		-nographic \
		-kernel $(OUT_DIR)/bzImage

# Run linux config menu and save output
.PHONY: linux-config
linux-config:
	rm $(CONFIG_DIR)/$(TARGET)/linux.config
	make $(CONFIG_DIR)/$(TARGET)/linux.config

$(OUT_DIR)/$(ARCH).bzImage: $(CACHE_DIR)/bzImage
	cp $(CACHE_DIR)/bzImage $(OUT_DIR)/$(ARCH).bzImage

$(OUT_DIR)/$(ARCH).eif: \
	$(BIN_DIR)/eif_build \
	$(CACHE_DIR)/bzImage \
	$(CACHE_DIR)/rootfs.cpio \
	$(CACHE_DIR)/linux.config
	$(call toolchain,$(USER)," \
		$(BIN_DIR)/eif_build \
			--kernel $(CACHE_DIR)/bzImage \
			--kernel_config $(CACHE_DIR)/linux.config \
			--cmdline 'reboot=k initrd=0x2000000$(,)3228672 root=/dev/ram0 panic=1 pci=off nomodules console=ttyS0 i8042.noaux i8042.nomux i8042.nopnp i8042.dumbkbd' \
			--ramdisk $(CACHE_DIR)/rootfs.cpio \
			--output $@; \
	")

$(FETCH_DIR)/aws-nitro-enclaves-sdk-bootstrap:
	$(call git_clone,$@,$(AWS_NITRO_DRIVER_REPO),$(AWS_NITRO_DRIVER_REF))

$(FETCH_DIR)/aws-nitro-enclaves-image-format:
	$(call git_clone,$@,$(AWS_EIF_REPO),$(AWS_EIF_REF))

$(KEY_DIR)/$(LINUX_KEY).asc:
	$(call fetch_pgp_key,$(LINUX_KEY))

$(FETCH_DIR)/linux-$(LINUX_VERSION).tar.sign:
	curl --url $(LINUX_SERVER)/linux-$(LINUX_VERSION).tar.sign --output $@

$(FETCH_DIR)/linux-$(LINUX_VERSION).tar.xz:
	curl --url $(LINUX_SERVER)/linux-$(LINUX_VERSION).tar.xz --output $@

$(FETCH_DIR)/linux-$(LINUX_VERSION).tar: \
	$(FETCH_DIR)/linux-$(LINUX_VERSION).tar.xz
	xz -kd $@.xz

$(FETCH_DIR)/linux-$(LINUX_VERSION): toolchain \
	$(FETCH_DIR)/linux-$(LINUX_VERSION).tar \
	$(FETCH_DIR)/linux-$(LINUX_VERSION).tar.sign \
	$(KEY_DIR)/$(LINUX_KEY).asc
	$(call toolchain,$(USER), " \
		unset FAKETIME; \
		gpg --import $(KEY_DIR)/$(LINUX_KEY).asc && \
		gpg --verify $@.tar.sign $@.tar && \
		cd $(FETCH_DIR) && \
		tar xf linux-$(LINUX_VERSION).tar; \
	")

$(CONFIG_DIR)/$(TARGET)/linux.config: \
	$(FETCH_DIR)/linux-$(LINUX_VERSION)
	$(call toolchain,$(USER)," \
		cp $@ $(FETCH_DIR)/linux-$(LINUX_VERSION)
		cd $(FETCH_DIR)/linux-$(LINUX_VERSION) && \
		make menuconfig && \
		cp .config $@; \
	")


$(CACHE_DIR)/linux.config:
	cp $(CONFIG_DIR)/$(TARGET)/linux.config $@

$(CACHE_DIR)/rootfs:
	mkdir -p $@

$(CACHE_DIR)/rootfs/init: \
	$(CACHE_DIR)/rootfs
	$(call toolchain,$(USER)," \
		unset FAKETIME; \
		cd $(SRC_DIR)/init && \
		RUSTFLAGS='-C target-feature=+crt-static' cargo build \
			--target $(ARCH)-unknown-linux-gnu \
			--release && \
		cd - && \
		cp \
			$(SRC_DIR)/init/target/$(ARCH)-unknown-linux-gnu/release/init \
			$@ && \
		touch -hcd "@0" $@ \
	")

$(BIN_DIR)/gen_init_cpio: \
	$(FETCH_DIR)/linux-$(LINUX_VERSION)
	$(call toolchain,$(USER)," \
		cd $(FETCH_DIR)/linux-$(LINUX_VERSION) && \
		gcc usr/gen_init_cpio.c -o $$HOME/$@ \
	")

$(BIN_DIR)/gen_initramfs.sh: \
	$(FETCH_DIR)/linux-$(LINUX_VERSION) \
	$(FETCH_DIR)/linux-$(LINUX_VERSION)/usr/gen_initramfs.sh
	cat $(FETCH_DIR)/linux-$(LINUX_VERSION)/usr/gen_initramfs.sh \
	| sed 's:usr/gen_init_cpio:gen_init_cpio:g' \
	> $@
	chmod +x $@

$(CACHE_DIR)/rootfs.list: \
	$(CONFIG_DIR)/$(TARGET)/rootfs.list
	cp $(CONFIG_DIR)/$(TARGET)/rootfs.list $(CACHE_DIR)/rootfs.list

$(CACHE_DIR)/rootfs.cpio: \
	$(CACHE_DIR)/rootfs \
	$(CACHE_DIR)/rootfs.list \
	$(CACHE_DIR)/rootfs/init \
	$(FETCH_DIR)/linux-$(LINUX_VERSION) \
	$(BIN_DIR)/gen_init_cpio \
	$(BIN_DIR)/gen_initramfs.sh
ifeq ($(TARGET), aws)
	$(MAKE) TARGET=$(TARGET) $(CACHE_DIR)/rootfs/nsm.ko
endif
	$(call toolchain,$(USER)," \
		find $(CACHE_DIR)/rootfs \
			-mindepth 1 \
			-execdir touch -hcd "@0" "{}" + && \
		gen_initramfs.sh -o $@ $(CACHE_DIR)/rootfs.list && \
		cpio -itv < $@ && \
		sha256sum $@; \
	")

$(CACHE_DIR)/bzImage: \
	$(CONFIG_DIR)/$(TARGET)/linux.config \
	$(FETCH_DIR)/linux-$(LINUX_VERSION) \
	$(CACHE_DIR)/rootfs.cpio
	$(call toolchain,$(USER)," \
		cd $(FETCH_DIR)/linux-$(LINUX_VERSION) && \
		cp /home/build/$(CONFIG_DIR)/$(TARGET)/linux.config .config && \
		make olddefconfig && \
		make -j$(CPUS) ARCH=$(ARCH) bzImage && \
		cp arch/$(ARCH)/boot/bzImage /home/build/$@ && \
		sha256sum /home/build/$@; \
	")

$(BIN_DIR)/eif_build: \
	$(FETCH_DIR)/aws-nitro-enclaves-image-format
	$(call toolchain,$(USER)," \
		unset FAKETIME; \
		cd $(FETCH_DIR)/aws-nitro-enclaves-image-format \
		&& CARGO_HOME=$(CACHE_DIR)/cargo cargo build --example eif_build \
		&& cp target/debug/examples/eif_build /home/build/$@; \
	")

$(CACHE_DIR)/rootfs/nsm.ko: \
	$(FETCH_DIR)/linux-$(LINUX_VERSION) \
	$(FETCH_DIR)/aws-nitro-enclaves-sdk-bootstrap
	$(call toolchain,$(USER)," \
		cd $(FETCH_DIR)/linux-$(LINUX_VERSION) && \
		cp /home/build/$(CONFIG_DIR)/$(TARGET)/linux.config .config && \
		make olddefconfig && \
		make -j$(CPUS) ARCH=$(ARCH) bzImage && \
		make -j$(CPUS) ARCH=$(ARCH) modules_prepare && \
		cd $(FETCH_DIR)/aws-nitro-enclaves-sdk-bootstrap/ \
		&& make \
			-C $(FETCH_DIR)/linux-$(LINUX_VERSION) \
			M=$(FETCH_DIR)/aws-nitro-enclaves-sdk-bootstrap/nsm-driver \
		&& cp nsm-driver/nsm.ko $@; \
		touch -hcd "@0" $@ \
	")
