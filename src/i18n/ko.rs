use super::Msg;

pub fn text(msg: Msg) -> &'static str {
    match msg {
        Msg::LabelQuestion => "질문",
        Msg::LabelCommand => "명령",
        Msg::LabelExplain => "설명",
        Msg::LabelWarning => "경고",
        Msg::LabelNotes => "참고",
        Msg::PlaceholderQuestion => "하고 싶은 작업을 입력",
        Msg::PlaceholderFollowUp => "수정 요청 입력, 또는 명령에서 Enter로 확정",
        Msg::PlaceholderLoginPaste => "브라우저가 다른 기기에 있으면 이동된 URL을 여기에 붙여넣기",
        Msg::HintQuestion => "Enter 제출  Tab 명령  Esc 종료",
        Msg::HintCommand => "Enter 확정  Ctrl+J 줄바꿈  Ctrl+E 설명  Tab 질문  Esc 종료",
        Msg::HintRequesting => "Esc 취소",
        Msg::HintLogin => "Enter 로그인  Esc 종료",
        Msg::StatusGenerating => "생성 중",
        Msg::StatusRevising => "수정 중",
        Msg::StatusExplaining => "설명 생성 중",
        Msg::StatusCancelled => "요청 취소됨",
        Msg::StatusExplanationStale => "명령어 수정됨. Ctrl+E로 설명 갱신",
        Msg::StatusConfirmDanger => {
            "위험도 높은 명령. Enter를 한 번 더 누르면 확정, 다른 키는 편집 계속"
        }
        Msg::StatusTooSmall => "터미널 크기 부족 (최소 {w}x{h})",
        Msg::StatusFullscreenUnavailable => "전체화면 모드 미지원 버전. Inline 모드로 실행",
        Msg::UsingChatGptPlan => "Using ChatGPT plan",
        Msg::PlanNotice => {
            "You're using your ChatGPT plan. 사용량 관리: https://chatgpt.com/settings/usage"
        }
        Msg::LoginRequired => "로그인 필요. Enter: Continue with ChatGPT",
        Msg::LoginWaiting => "브라우저 로그인 대기 중",
        Msg::LoginOpenUrl => "로그인 URL:",
        Msg::LoginPasteHint => {
            "브라우저가 다른 기기에서 실행 중이면 이동된 주소(http://127.0.0.1 로 시작)를 붙여넣고 Enter"
        }
        Msg::LoginSshHint => {
            "원격 장치에서는 포트 포워딩도 가능: ssh -L {port}:127.0.0.1:{port} <host>"
        }
        Msg::LoginBrowserFailed => "브라우저 실행 실패. URL을 직접 열어 주세요.",
        Msg::LoginSuccess => "로그인 완료: {email}",
        Msg::LoginPlanDisabled => {
            "로그인은 완료됐지만 ChatGPT 플랜 사용 권한이 없음. `howtui login`을 다시 실행해 \"Use your ChatGPT plan\"을 허용"
        }
        Msg::LoginCancelled => "로그인 취소됨",
        Msg::LoginInvalidPaste => "이번 로그인 시도의 리다이렉트 URL이 아님",
        Msg::LoginStateMismatch => "로그인 응답이 이번 시도와 불일치 (state 불일치). 다시 시도",
        Msg::LoginDenied => "브라우저에서 로그인이 거부됨",
        Msg::LoginTimeout => "로그인 시간 초과. 다시 시도",
        Msg::PrivacyNotice => {
            "howtui는 ChatGPT 계정(Sign in with ChatGPT)으로 로그인하고, 발급된 토큰을 이 기기에 저장 \
             (OS 보안 저장소, 또는 본인만 읽을 수 있는 파일). OpenAI로부터 계정 이메일과 이름을 수신. \
             요청 제출 시 질문, 현재 명령어(사용자 수정본과 시작 시점의 쉘 입력란 포함), OS, CPU 아키텍처, 쉘 이름을 \
             OpenAI로 전송. 환경변수, 파일, 쉘 히스토리, 작업 디렉터리는 전송하지 않으며 명령어를 실행하지 않음."
        }
        Msg::LogoutDone => "로그아웃 완료",
        Msg::LogoutForgetDone => "로그아웃과 계정 등록 정보 삭제 완료",
        Msg::LogoutRevokeUnconfirmed => {
            "로컬 로그아웃 완료. OpenAI 측 토큰 폐기는 미확인. ChatGPT 설정 > Security and login에서 howtui 연결 해제 가능"
        }
        Msg::LogoutNotSignedIn => "로그인 상태 아님",
        Msg::AuthStatusSignedIn => "로그인: {email}",
        Msg::AuthStatusSignedOut => "로그인 상태 아님. `howtui login` 실행",
        Msg::AuthStatusPlan => "ChatGPT 플랜 사용: 허용",
        Msg::AuthStatusPlanDisabled => "ChatGPT 플랜 사용: 미허용",
        Msg::AuthStatusExpires => "액세스 토큰 만료: {time}",
        Msg::AuthStatusStore => "자격증명 저장소: {store}",
        Msg::StoreFileFallback => {
            "OS 보안 저장소 사용 불가. 본인만 읽을 수 있는 파일에 토큰 저장: {path}"
        }
        Msg::ErrAuthRequired => "로그인 필요. `howtui login` 실행 (TUI에서는 Enter)",
        Msg::ErrAuthFailed => "인증 실패. `howtui login`으로 다시 로그인",
        Msg::ErrNotEligible => {
            "이 계정 또는 워크스페이스에서 ChatGPT 플랜 사용 불가. 플랜과 워크스페이스 확인"
        }
        Msg::ErrClientNotEnabled => {
            "howtui의 ChatGPT 플랜 사용이 아직 활성화되지 않음. 활성화 전까지 사용 불가"
        }
        Msg::ErrUsageLimit => {
            "ChatGPT 플랜 사용량 한도 도달. 사용량 관리: https://chatgpt.com/settings/usage"
        }
        Msg::ErrRegionOrPolicy => "정책 또는 지역 제한으로 요청 차단: {detail}",
        Msg::ErrPermissionConfig => "로그인 권한 구성 불일치 ({code}). 다시 로그인",
        Msg::ErrUnsupported => "모델 또는 요청 기능 미지원 ({param}). 모델 설정 확인",
        Msg::ErrModelNotFound => "모델 없음. `provider.chatgpt.model` 설정 확인 또는 삭제",
        Msg::ErrTransient => "서비스 일시 장애. 잠시 후 다시 시도",
        Msg::ErrNetwork => "네트워크 오류: {detail}. 연결 확인 후 다시 시도",
        Msg::ErrTimeout => "요청 시간 초과. 다시 시도",
        Msg::ErrInvalidResponse => "모델 응답 형식 해석 실패. 다시 시도",
        Msg::ErrNoTerminal => {
            "사용 가능한 터미널(TTY) 없음. 비대화형 실행: howtui --no-tui \"<질문>\""
        }
        Msg::ErrConfig => "설정 오류: {detail}",
        Msg::ErrInternal => "내부 오류: {detail}",
        Msg::ErrNoCommand => "모델 응답에 명령어 없음. 쉘 명령어를 요청",
        Msg::ErrNoQuestion => "--no-tui에는 질문 필요",
        Msg::RiskHigh => "위험 높음",
        Msg::RiskCaution => "주의",
        Msg::CatRecursiveDelete => "재귀 삭제",
        Msg::CatDelete => "파일 삭제",
        Msg::CatDisk => "디스크 직접 쓰기 또는 초기화",
        Msg::CatPermission => "권한 재귀 변경",
        Msg::CatRemoteScript => "다운로드한 스크립트 실행",
        Msg::CatMassKill => "다수 프로세스 종료",
        Msg::CatPower => "시스템 종료, 재시작, 서비스 중지",
        Msg::CatPrivilege => "관리자 권한 실행",
        Msg::CatOverwrite => "파일 덮어쓰기 가능",
        Msg::CatGitHistory => "git 이력 또는 변경 사항 손실 가능",
        Msg::CatSystemConfig => "시스템 설정 변경",
        Msg::CatForkBomb => "fork bomb",
        Msg::DoctorTitle => "howtui doctor",
        Msg::DoctorPass => "정상",
        Msg::DoctorWarn => "경고",
        Msg::DoctorFail => "실패",
        Msg::DoctorInfo => "정보",
        Msg::DoctorSkip => "생략",
        Msg::DoctorSummaryOk => "실패 항목 없음",
        Msg::DoctorSummaryFail => "실패 항목 {count}개",
        Msg::HelpAbout => {
            "AI 터미널 명령어 도우미: 작업을 설명하면 명령어를 제안하고, 편집 후 stdout으로 전달"
        }
        Msg::HelpQuestion => "바로 보낼 질문",
        Msg::HelpNoTui => "TUI 없이 1회 생성 (명령어는 stdout, 설명은 stderr)",
        Msg::HelpShell => "대상 쉘 (기본: 쉘 통합 정보, 없으면 OS 기본값)",
        Msg::HelpHeight => "Inline 화면 높이 (3-40줄)",
        Msg::HelpMode => "화면 모드",
        Msg::HelpLogin => "ChatGPT 계정으로 로그인",
        Msg::HelpLogout => "로그아웃과 세션 폐기",
        Msg::HelpAuth => "인증 명령",
        Msg::HelpAuthStatus => "로그인 상태 표시",
        Msg::HelpDoctor => "환경과 설정 점검",
        Msg::HelpDoctorJson => "JSON 출력",
        Msg::HelpNoBrowser => "브라우저를 열지 않고 URL만 표시 (SSH, 헤드리스 장치용)",
        Msg::HelpPort => "고정 로컬 콜백 포트 (SSH 포트 포워딩용)",
        Msg::HelpForget => "계정 등록 정보도 삭제",
        Msg::HelpHelp => "도움말 출력",
        Msg::HelpVersion => "버전 출력",
        Msg::HelpAfter => {
            "종료 코드: 0 확정, 2 사용법, 3 터미널 없음, 4 인증, 5 플랜과 사용량, 6 네트워크, 7 Provider, 8 설정, 130 취소\n\
             모델 전송 정보: 질문, 현재 명령어, OS, CPU 아키텍처, 쉘 이름\n\
             위험 경고는 규칙 기반 참고 정보이며 보안 보장 수단이 아님. howtui는 명령어를 실행하지 않음"
        }
    }
}
