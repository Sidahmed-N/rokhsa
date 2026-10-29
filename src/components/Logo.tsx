import logo from "@/assets/logo.png";

/**
 * The Rokhsa mark: a steering wheel behind a licence card. Rokhsa (رخصة) is
 * the licence; the wheel is what it is for.
 *
 * A bundled file, imported so Vite ships it inside the app: it renders with
 * no network, like the fonts. The same image is src-tauri/icons/source.png,
 * which every window and installer icon is generated from — replace both.
 *
 * Not mirrored in Arabic: it is a mark, not text, and a logo that flips
 * with the language is two logos.
 */
export default function Logo({ className }: { className?: string | undefined }) {
  return <img src={logo} alt="" aria-hidden="true" draggable={false} className={className} />;
}
