SIGN="C:\\Program Files (x86)\\Windows Kits\\10\\App Certification Kit\\signtool.exe"

.DUMMY: debug
debug:
	cargo build

.DUMMY: release
release:
	cargo build --release

.DUMMY: sign
sign: release
	del lyric-check.exe
	copy "target\\release\\lyric-check.exe" lyric-check-to-be-signed.exe
	$(SIGN) sign /a /tr http://timestamp.globalsign.com/tsa/r6advanced1 /td SHA256 /v lyric-check-to-be-signed.exe
	ren lyric-check-to-be-signed.exe lyric-check.exe