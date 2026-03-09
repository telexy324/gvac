import { FormEvent, useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Eye, EyeOff, RefreshCw, ShieldCheck } from "lucide-react";
import { toast } from "sonner";

import { login, captcha } from "@/api/user";
import { setApiAuth } from "@/lib/api";
import { useAuthStore } from "@/stores/useAuthStore";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

type LoginForm = {
  username: string;
  password: string;
  captcha: string;
  captchaId: string;
};

export default function LoginPage() {
  const navigate = useNavigate();
  const token = useAuthStore((s) => s.token);
  const setToken = useAuthStore((s) => s.setToken);
  const setUserInfo = useAuthStore((s) => s.setUserInfo);

  const [showPwd, setShowPwd] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [captchaImg, setCaptchaImg] = useState("");
  const [form, setForm] = useState<LoginForm>({
    username: "",
    password: "",
    captcha: "",
    captchaId: ""
  });

  useEffect(() => {
    if (token) {
      navigate("/layout", { replace: true });
    }
  }, [token, navigate]);

  const fetchCaptcha = async () => {
    try {
      const res = await captcha({});
      if (res?.code === 0 && res.data) {
        setCaptchaImg(res.data.picPath);
        setForm((prev) => ({ ...prev, captchaId: res.data!.captchaId, captcha: "" }));
      }
    } catch {
      toast.error("获取验证码失败");
    }
  };

  useEffect(() => {
    void fetchCaptcha();
  }, []);

  const onChange = <K extends keyof LoginForm>(key: K, value: LoginForm[K]) => {
    setForm((prev) => ({ ...prev, [key]: value }));
  };

  const onSubmit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (form.username.length < 5 || form.password.length < 6 || form.captcha.length < 5) {
      toast.error("请填写正确的登录信息");
      void fetchCaptcha();
      return;
    }

    setSubmitting(true);
    try {
      const res = await login(form);
      if (res?.code === 0 && res.data) {
        const nextToken = String(res.data.token ?? "");
        const nextUser = (res.data.user as Record<string, unknown>) ?? null;
        setToken(nextToken);
        setUserInfo(nextUser);
        await setApiAuth(nextToken, String(nextUser?.ID ?? ""));
        navigate("/layout", { replace: true });
      } else {
        void fetchCaptcha();
      }
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="relative min-h-screen overflow-hidden bg-slate-950 px-4 py-8 sm:px-6 lg:px-8">
      <div className="pointer-events-none absolute inset-0">
        <div className="absolute -left-24 top-[-140px] h-80 w-80 rounded-full bg-cyan-400/25 blur-3xl" />
        <div className="absolute -right-24 top-1/3 h-72 w-72 rounded-full bg-emerald-300/20 blur-3xl" />
        <div className="absolute bottom-[-120px] left-1/3 h-80 w-80 rounded-full bg-indigo-400/20 blur-3xl" />
      </div>

      <div className="relative mx-auto flex min-h-[calc(100vh-4rem)] w-full max-w-6xl items-center">
        <div className="grid w-full overflow-hidden rounded-3xl border border-white/10 bg-white/5 shadow-2xl backdrop-blur-xl lg:grid-cols-[1.1fr_1fr]">
          <section className="hidden flex-col justify-between border-r border-white/10 p-10 lg:flex">
            <div>
              <p className="inline-flex items-center gap-2 rounded-full border border-cyan-300/40 bg-cyan-400/10 px-4 py-1 text-xs font-medium tracking-wide text-cyan-100">
                <ShieldCheck className="h-3.5 w-3.5" />
                Secure Access Portal
              </p>
              <h1 className="mt-6 text-4xl font-semibold leading-tight text-white">
                Gin Admin
                <br />
                管理后台
              </h1>
              <p className="mt-4 max-w-md text-sm leading-7 text-slate-300">
                简洁、快速、可靠的系统入口。请使用账号密码与验证码登录以继续访问管理功能。
              </p>
            </div>
            <div className="rounded-2xl border border-white/15 bg-white/5 p-5 text-sm text-slate-300">
              登录异常时可先刷新验证码；若数据库尚未初始化，请使用“前往初始化”。
            </div>
          </section>

          <section className="p-6 sm:p-10">
            <Card className="border-0 bg-transparent shadow-none">
              <CardHeader className="space-y-2 px-0 pt-0">
                <CardTitle className="text-3xl font-semibold tracking-tight text-white">欢迎登录</CardTitle>
                <CardDescription className="text-slate-300">React + shadcn/ui 后台管理入口</CardDescription>
              </CardHeader>

              <CardContent className="px-0 pb-0">
                <form className="space-y-4" onSubmit={onSubmit}>
                  <Input
                    placeholder="用户名"
                    value={form.username}
                    onChange={(e) => onChange("username", e.target.value)}
                    className="h-11 border-white/20 bg-white/10 text-white placeholder:text-slate-400"
                  />

                  <div className="relative">
                    <Input
                      placeholder="密码"
                      type={showPwd ? "text" : "password"}
                      value={form.password}
                      onChange={(e) => onChange("password", e.target.value)}
                      className="h-11 border-white/20 bg-white/10 pr-12 text-white placeholder:text-slate-400"
                    />
                    <button
                      type="button"
                      className="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 transition-colors hover:text-white"
                      onClick={() => setShowPwd((v) => !v)}
                      aria-label={showPwd ? "隐藏密码" : "显示密码"}
                    >
                      {showPwd ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                    </button>
                  </div>

                  <div className="flex items-center gap-2">
                    <Input
                      placeholder="验证码"
                      value={form.captcha}
                      onChange={(e) => onChange("captcha", e.target.value)}
                      className="h-11 border-white/20 bg-white/10 text-white placeholder:text-slate-400"
                    />
                    <button
                      type="button"
                      onClick={() => void fetchCaptcha()}
                      className="flex h-11 w-[124px] items-center justify-center overflow-hidden rounded-md border border-white/20 bg-white/10 transition-colors hover:bg-white/15"
                      aria-label="刷新验证码"
                    >
                      {captchaImg ? (
                        <img src={captchaImg} alt="captcha" className="h-full w-full object-cover" />
                      ) : (
                        <RefreshCw className="h-4 w-4 animate-spin text-slate-300" />
                      )}
                    </button>
                  </div>

                  <div className="grid grid-cols-2 gap-3 pt-1">
                    <Button
                      type="submit"
                      className="h-11 bg-cyan-500 text-slate-950 hover:bg-cyan-400"
                      disabled={submitting}
                    >
                      {submitting ? "登录中..." : "登录"}
                    </Button>
                  </div>
                </form>
              </CardContent>
            </Card>
          </section>
        </div>
      </div>
    </div>
  );
}
