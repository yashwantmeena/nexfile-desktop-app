import { ShieldCheck } from "lucide-react";

interface PrivacyBannerProps {
  onLearnMore: () => void;
}

export function PrivacyBanner({ onLearnMore }: PrivacyBannerProps) {
  return (
    <section className="privacy-banner">
      <span className="privacy-icon"><ShieldCheck /></span>
      <div><strong>Privacy First</strong><p>LocalMind AI runs 100% on your device. No data leaves your computer. Tags are generated using on-device models.</p></div>
      <button type="button" onClick={onLearnMore}>Learn more</button>
    </section>
  );
}
