.PHONY: keys
keys: \
	$(KEY_DIR)/$(LINUX_KEY).asc \
	$(KEY_DIR)/$(BUSYBOX_KEY).asc

$(KEY_DIR)/$(LINUX_KEY).asc:
	$(call fetch_pgp_key,$(LINUX_KEY))

$(KEY_DIR)/$(BUSYBOX_KEY).asc:
	$(call fetch_pgp_key,,$(BUSYBOX_KEY))

define fetch_pgp_key
	mkdir -p $(KEY_DIR) && \
	$(toolchain) ' \
		for server in \
    	    ha.pool.sks-keyservers.net \
    	    hkp://keyserver.ubuntu.com:80 \
    	    hkp://p80.pool.sks-keyservers.net:80 \
    	    pgp.mit.edu \
    	; do \
			echo "Trying: $${server}"; \
    	   	gpg \
    	   		--recv-key \
    	   		--keyserver "$${server}" \
    	   		--keyserver-options timeout=10 \
    	   		--recv-keys "$(1)" \
    	   	&& break; \
    	done; \
		gpg --export -a $(1) > $(KEY_DIR)/$(1).asc; \
	'
endef
