# Changelog

모든 주요 변경 사항을 기록합니다.

## v0.1.1 - 2025-12-30

### Added
- x86_64 USB xHCI HID 키보드 입력 경로 추가
- x86_64 legacy virtio 키보드 입력 경로 추가
- HHDM 기반 DMA 주소 변환 지원
- AArch64 번들 빌드 스크립트 (`tools/build_bundle_arm.sh`)

### Changed
- x86_64 ISO 빌드 플로우 정리(UEFI/BIOS 하이브리드)
- QEMU 실행 스크립트에 virtio 키보드 옵션 기본 추가
- 부팅/입력 관련 문서 업데이트

### Fixed
- PS/2 포트 입력 처리 경고 제거
- 부팅 정보(BootInfo) 초기화 누락 보완
