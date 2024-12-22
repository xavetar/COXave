class ASCII(object):

    @staticmethod
    def __new__(*args, **kwargs):
        pass

    def __init__(self, *args, **kwargs):
        pass

    @staticmethod
    def is_ascii(array: bytes) -> bool:
        """
        The function checks the source byte sequence for coherence with the ASCII encoding

        :param array: Encoded byte/s sequence
        :return: Result of checking the sequence for compliance with the encoding format
        """
        pass

class UTF8(object):

    @staticmethod
    def __new__(*args, **kwargs):
        pass

    def __init__(self, *args, **kwargs):
        pass

    @staticmethod
    def is_utf8(array: bytes) -> bool:
        """
        The function checks the source byte sequence for coherence with the UTF-8 encoding

        :param array: Encoded byte/s sequence
        :return: Result of checking the sequence for compliance with the encoding format
        """
        pass

class UTF16(object):

    @staticmethod
    def __new__(*args, **kwargs):
        pass

    def __init__(self, *args, **kwargs):
        pass

    @staticmethod
    def is_utf16(array: bytes, endian: bool, omp: bool, only: bool) -> bool:
        """
        The function checks the source byte sequence for coherence with the UTF-16 encoding (from 0x00 to 0x7F)

        :param array: Encoded bytes sequence
        :param endian: Byte order of the encoded bytes sequence (0:False - BE, 1:True - LE)
        :param omp: Enable/Disable over multilingual plane
        :param only: Enable/Disable selected multilingual plane (OMP + Only = Only surrogates, Only - only basic pairs)
        :return: Result of checking the sequence for compliance with the encoding format
        """
        pass


class UTF32(object):

    @staticmethod
    def __new__(*args, **kwargs):
        pass

    def __init__(self, *args, **kwargs):
        pass

    @staticmethod
    def is_utf32(array: bytes, endian: bool) -> bool:
        """
        The function checks the source byte sequence for coherence with the ASCII encoding (from 0x00 to 0x7F)

        :param array: Encoded bytes sequence
        :param endian: Byte order of the encoded bytes sequence (0:False - BE, 1:True - LE)
        :return: Result of checking the sequence for compliance with the encoding format
        """
        pass

__all__ = [
    'ASCII',
    'UTF8',
    'UTF16',
    'UTF32',
]
