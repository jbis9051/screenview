export default function formatID(id: string) {
    if (!id.match(/^[\d\s]+$/)) {
        return id;
    }
    let format = id.substring(0, 13).replaceAll(/\D/g, '');
    if (format.length > 9) {
        format = `${format[0]} ${format
            .substring(1)
            .replaceAll(/(\d{3})/g, '$1 ')}`;
    } else {
        format = format.replaceAll(/(\d{3})/g, '$1 ');
    }
    return format.trim();
}
