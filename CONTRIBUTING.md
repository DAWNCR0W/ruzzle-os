# Contributing

Ruzzle OS에 기여해주셔서 감사합니다. 아래 규칙을 지켜주세요.

## 브랜치 규칙 (Git Flow)
- `main`: 배포용, 직접 푸시 금지
- `develop`: 다음 릴리즈 통합 브랜치
- 브랜치 이름: `type/description`
  - `feature/*`, `fix/*`, `release/*`, `hotfix/*`, `refactor/*`, `chore/*`

## 커밋 메시지 규칙
형식: `type: :gitmoji: 설명`
- 예시: `feat: ✨ add usb keyboard input`

## 코드 규칙
- 아키텍처: Presentation → Domain ← Data (클린 아키텍처 준수)
- 네이밍/포맷: 프로젝트 규칙 준수
- 경고/클리피 오류는 허용하지 않습니다.

## 테스트
- 가능한 경우 `cargo test` 또는 프로젝트 테스트를 수행해주세요.
- 커버리지 기준이 있는 경우 이를 만족해야 합니다.

## PR 작성
- 작은 단위로 분리
- 변경 요약 + 테스트 결과 포함
- 관련 이슈 연결
