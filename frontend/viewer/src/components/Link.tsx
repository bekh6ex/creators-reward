export function Link(prop: {href: string, children: string, external?: boolean}) {
    const extra = prop.external ? {target: "_blank"} : {}
    return <a className="link" href={prop.href} {...extra}>{prop.children}</a>;
}
