#include <windows.h>
#include <processthreadsapi.h>
#include <sstream>

extern "C" void log_error_in_rust(const char* error);

char* GetWin32ErrorMessage(DWORD dw) noexcept
{
    char* lpMsgBuf = nullptr;

    ::FormatMessageA(
        FORMAT_MESSAGE_ALLOCATE_BUFFER |
        FORMAT_MESSAGE_FROM_SYSTEM |
        FORMAT_MESSAGE_IGNORE_INSERTS,
        NULL,
        dw,
        MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT),
        (LPSTR) &lpMsgBuf,
        0, NULL);

    return lpMsgBuf;
}

void LogLastErrorMessage(const char* taskname) noexcept try
{
    DWORD dw = ::GetLastError();
    std::stringstream ss;
    std::unique_ptr<char, decltype(&::LocalFree)> buf(
        GetWin32ErrorMessage(dw),
        &::LocalFree);

    ss << "Failed '" << taskname << "' with error " << dw;
    if (buf.get())
    {
        ss << ": " << buf.get();
    }
    ss << ".";

    log_error_in_rust(ss.str().c_str());
}
catch (...)
{
}

extern "C" bool Win32_EnableTerminalAnsiSupport(void) noexcept
{
    #define LOG_AND_RETURN_IF_FALSE(expr) \
    if (!(expr)) \
    { \
        LogLastErrorMessage(#expr); \
        return false; \
    }

    std::unique_ptr<std::remove_pointer<HANDLE>::type, 
        decltype(&::CloseHandle)> handle(
            ::CreateFileW(L"CONOUT$",
                            GENERIC_READ | GENERIC_WRITE,
                            FILE_SHARE_WRITE,
                            nullptr,
                            OPEN_EXISTING,
                            0,
                            nullptr),
                        &::CloseHandle);

    LOG_AND_RETURN_IF_FALSE(handle.get() != INVALID_HANDLE_VALUE);

    DWORD mode = 0;
    LOG_AND_RETURN_IF_FALSE(GetConsoleMode(handle.get(), &mode));

    if ((mode & ENABLE_VIRTUAL_TERMINAL_PROCESSING) == 0)
    {
        mode |= ENABLE_VIRTUAL_TERMINAL_PROCESSING;
        LOG_AND_RETURN_IF_FALSE(SetConsoleMode(handle.get(), mode));
    }

    return true;
}

extern "C" bool Win32_EnableMitigations(void) noexcept
{
    #define APPEND(a, b) #a #b

    #define SET_MITIGATION(MitigationPolicy, Settings) \
    { \
        if (!::SetProcessMitigationPolicy(MitigationPolicy, \
                                          &Settings, \
                                          sizeof(Settings))) \
        { \
            LogLastErrorMessage(APPEND(Set, MitigationPolicy)); \
            return false; \
        } \
    }

    PROCESS_MITIGATION_IMAGE_LOAD_POLICY imageLoadPolicy = {};
    imageLoadPolicy.NoRemoteImages = TRUE;
    imageLoadPolicy.NoLowMandatoryLabelImages = TRUE;
    SET_MITIGATION(ProcessImageLoadPolicy, imageLoadPolicy);

    PROCESS_MITIGATION_FONT_DISABLE_POLICY fontPolicy = {};
    fontPolicy.DisableNonSystemFonts = TRUE;
    SET_MITIGATION(ProcessFontDisablePolicy, fontPolicy);

    PROCESS_MITIGATION_DYNAMIC_CODE_POLICY dynamicCodePolicy = {};
    dynamicCodePolicy.ProhibitDynamicCode = TRUE;
    SET_MITIGATION(ProcessDynamicCodePolicy, dynamicCodePolicy);

    PROCESS_MITIGATION_CHILD_PROCESS_POLICY childProcessPolicy = {};
    childProcessPolicy.NoChildProcessCreation = TRUE;
    SET_MITIGATION(ProcessChildProcessPolicy, childProcessPolicy);

    PROCESS_MITIGATION_BINARY_SIGNATURE_POLICY sigPolicy = {};
    sigPolicy.MicrosoftSignedOnly = TRUE;
    SET_MITIGATION(ProcessSignaturePolicy, sigPolicy);

    PROCESS_MITIGATION_SYSTEM_CALL_DISABLE_POLICY syscallPolicy = {};
    syscallPolicy.DisallowWin32kSystemCalls = TRUE;
    SET_MITIGATION(ProcessSystemCallDisablePolicy, syscallPolicy);

    PROCESS_MITIGATION_STRICT_HANDLE_CHECK_POLICY handlePolicy = {};
    handlePolicy.HandleExceptionsPermanentlyEnabled = TRUE;
    handlePolicy.RaiseExceptionOnInvalidHandleReference = TRUE;
    SET_MITIGATION(ProcessStrictHandleCheckPolicy, handlePolicy);

    return true;
}
